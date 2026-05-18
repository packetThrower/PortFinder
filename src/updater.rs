//! GitHub Releases poll for "update available" indication. Fired
//! once per launch (background blocking task), feeds the footer pill
//! when a newer version is published than the one currently running.
//!
//! Channel-policy choice: if the current version carries a SemVer
//! pre-release suffix (e.g. `4.0.0-alpha.1`), we report prereleases
//! too — alpha users get notified of the next alpha. Users on a
//! stable build only get notified of newer stables. That matches
//! the homebrew tap / scoop bucket's parallel `@alpha` /
//! `-prerelease` channels: each release line keeps to itself.

use semver::Version;
use serde::Deserialize;
use std::time::Duration;

const RELEASES_URL: &str =
    "https://api.github.com/repos/packetThrower/PortFinder/releases";
// Allowlist for the `html_url` field on each release. The footer
// pill click handler passes that string straight to `cx.open_url`,
// which on macOS hands it to NSWorkspace and on Linux to xdg-open
// — both honour any URL scheme they know about, so a malformed
// `html_url` could in principle ship `file:///etc/passwd` or
// `javascript:` (in old browsers) into the user's default handler.
// HTTPS + rustls cert validation makes a network-level MITM
// implausible, but a GitHub supply-chain compromise wouldn't be
// caught at the transport layer. Pin the prefix as defence in
// depth: any release whose `html_url` doesn't start with this is
// dropped before it can reach the UI.
const RELEASES_HTML_PREFIX: &str =
    "https://github.com/packetThrower/PortFinder/releases/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// What `check_for_update` returns when a newer release is found.
#[derive(Clone, Debug)]
pub struct UpdateInfo {
    /// Tag-style version with the leading `v` stripped:
    /// `"4.0.0-alpha.2"`, `"4.1.0"`, etc.
    pub version: String,
    /// HTTPS URL of the GitHub Release page (the "View on GitHub"
    /// target for the footer pill's click handler).
    pub html_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
}

/// Hit the GitHub Releases REST endpoint, parse the response, and
/// return the highest release whose version is greater than
/// `current`. Returns `None` on:
///   - network error / timeout / non-2xx response
///   - JSON parse failure
///   - no releases newer than `current`
///   - `current` itself isn't a valid SemVer (shouldn't happen —
///     `version.txt` is parsed by the bump script, but defensive
///     handling means the footer pill silently skips on a malformed
///     CARGO_PKG_VERSION rather than panicking the GUI).
///
/// Synchronous on purpose: callers (`AppView::new`) run this on a
/// `tokio::task::spawn_blocking` worker so the HTTP call doesn't
/// stall either the gpui render thread or the tokio reactor.
pub fn check_for_update(current: &str) -> Option<UpdateInfo> {
    let current_ver = Version::parse(current).ok()?;
    let include_prerelease = !current_ver.pre.is_empty();

    let user_agent = format!("PortFinder/{}", current);
    let response = ureq::get(RELEASES_URL)
        .set("User-Agent", &user_agent)
        .set("Accept", "application/vnd.github+json")
        .timeout(REQUEST_TIMEOUT)
        .call()
        .ok()?;
    let releases: Vec<Release> = response.into_json().ok()?;

    let mut best: Option<(Version, &Release)> = None;
    for release in &releases {
        if release.draft || release.tag_name.is_empty() {
            continue;
        }
        // URL-prefix allowlist — see `RELEASES_HTML_PREFIX` for the
        // threat model. The check is at the start of the loop so a
        // bogus `html_url` can't even win the `best` slot, let alone
        // make it out to `cx.open_url`.
        if !release.html_url.starts_with(RELEASES_HTML_PREFIX) {
            continue;
        }
        let tag = release.tag_name.trim_start_matches('v');
        let Ok(ver) = Version::parse(tag) else {
            continue;
        };
        // Legacy-tag filter. Pre-4.x releases on this repo used a
        // calendar-style version (`v2026.4.26-1`) that parses as a
        // valid SemVer with `major = 2026`. SemVer's numeric
        // ordering means that comparison wins against any honest
        // `4.x.y` build — so without this guard the footer pill
        // perma-suggests "Update v2026.4.26 available". The
        // threshold needs to be high enough to leave room for
        // real future major bumps (5, 6, …) but low enough to
        // catch year-style tags; 1000 is well clear of both.
        if ver.major >= 1000 {
            continue;
        }
        // Channel-policy gate (see module docs).
        if !include_prerelease && !ver.pre.is_empty() {
            continue;
        }
        if ver <= current_ver {
            continue;
        }
        if best.as_ref().is_none_or(|(b, _)| &ver > b) {
            best = Some((ver, release));
        }
    }

    best.map(|(_, r)| UpdateInfo {
        version: r.tag_name.trim_start_matches('v').to_string(),
        html_url: r.html_url.clone(),
    })
}
