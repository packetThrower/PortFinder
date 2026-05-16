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
        let tag = release.tag_name.trim_start_matches('v');
        let Ok(ver) = Version::parse(tag) else {
            continue;
        };
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
