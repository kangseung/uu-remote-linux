//! Reported UU client version that follows the official Windows release.
//!
//! The viewer disables newer features and warns "被控端版本较低" once `x-param-vn`
//! falls behind the official Windows client. Recompiling for every NetEase release
//! is impractical, so a background task re-reads the official download page (and the
//! winget manifest for the exact build number) and lifts the reported version.
//!
//! `UU_CLIENT_VERSION` still wins over detection: an explicit value is never replaced.

use std::sync::{OnceLock, RwLock};
use std::time::Duration;

static DETECTED: RwLock<Option<String>> = RwLock::new(None);
static STARTED: OnceLock<()> = OnceLock::new();

const DOWNLOAD_PAGE: &str = "https://uuyc.163.com/download/";
const WINGET_API: &str =
    "https://api.github.com/repos/microsoft/winget-pkgs/contents/manifests/n/NetEase/UURemote";

/// Version reported in `x-param-vn`: explicit override, else the detected official
/// version, else the version compiled in.
pub fn current() -> String {
    if let Some(v) = std::env::var("UU_CLIENT_VERSION").ok().filter(|v| !v.is_empty()) {
        return v;
    }
    DETECTED.read().ok().and_then(|v| v.clone())
        .unwrap_or_else(|| crate::auth::DEFAULT_CLIENT_VERSION.to_string())
}

/// Starts the detection loop once. Failures keep the last known version and retry sooner.
pub fn spawn_refresh() {
    if std::env::var("UU_CLIENT_VERSION").ok().filter(|v| !v.is_empty()).is_some() { return; }
    if STARTED.set(()).is_err() { return; }
    tokio::spawn(async move {
        loop {
            let delay = match fetch_latest().await {
                Ok(latest) => {
                    let previous = current();
                    if is_newer(&latest, &previous) {
                        if let Ok(mut slot) = DETECTED.write() { *slot = Some(latest.clone()); }
                        log::info!("Reported client version updated: {previous} -> {latest}");
                    } else {
                        log::info!("Reported client version current: {previous} (official {latest})");
                    }
                    Duration::from_secs(6 * 3600)
                }
                Err(e) => {
                    log::warn!("Client version check failed: {e}");
                    Duration::from_secs(3600)
                }
            };
            tokio::time::sleep(delay).await;
        }
    });
}

/// Newest version advertised by the official page and the winget manifest. The page
/// carries `major.minor.patch` only; the manifest adds the official build number.
async fn fetch_latest() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!("uu-remote-linux/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();
    if let Ok(html) = get_text(&client, DOWNLOAD_PAGE).await {
        if let Some(v) = parse_windows_version(&html) {
            candidates.push(v);
        }
    }
    if let Ok(body) = get_text(&client, WINGET_API).await {
        if let Some(v) = parse_winget_versions(&body) {
            candidates.push(v);
        }
    }

    candidates.into_iter().max_by(|a, b| rank(a).cmp(&rank(b)))
        .ok_or_else(|| "no official version source answered".to_string())
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    response.error_for_status().map_err(|e| e.to_string())?
        .text().await.map_err(|e| e.to_string())
}

/// The download page embeds schema.org JSON-LD per platform; take the Windows entry.
fn parse_windows_version(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(at) = rest.find("\"operatingSystem\":\"Windows\"") {
        let tail = &rest[at..];
        // The page is UTF-8 with Chinese text: never cut a multi-byte character.
        let mut end = tail.len().min(400);
        while !tail.is_char_boundary(end) { end -= 1; }
        let window = &tail[..end];
        if let Some(start) = window.find("\"softwareVersion\":\"") {
            let v = &window[start + "\"softwareVersion\":\"".len()..];
            let value = &v[..v.find('"')?];
            let parts = rank(value)?;
            if parts.len() == 3 {
                return Some(value.to_string());
            }
        }
        rest = &rest[at + 1..];
    }
    None
}

/// Winget manifest directories are named `major.minor.patch.build`; the highest is the
/// last release published for Windows.
fn parse_winget_versions(json: &str) -> Option<String> {
    let entries: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut newest: Option<String> = None;
    for entry in entries.as_array()? {
        let name = entry.get("name").and_then(|n| n.as_str())?;
        if rank(name).map(|p| p.len()) != Some(4) { continue; }
        if newest.as_deref().map_or(true, |best| is_newer(name, best)) {
            newest = Some(name.to_string());
        }
    }
    newest
}

fn rank(version: &str) -> Option<Vec<u64>> {
    let parts: Option<Vec<u64>> = version.split('.').map(|p| p.parse::<u64>().ok()).collect();
    let parts = parts?;
    matches!(parts.len(), 3 | 4).then_some(parts)
}

/// Segment-wise numeric comparison, padding a three-part version with a zero build.
fn is_newer(candidate: &str, baseline: &str) -> bool {
    let pad = |v: &str| { let mut p = rank(v).unwrap_or_default(); while p.len() < 4 { p.push(0); } p };
    pad(candidate) > pad(baseline)
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_winget_versions, parse_windows_version};

    #[test]
    fn reads_the_windows_entry_not_other_platforms() {
        let html = r#"[{"name":"网易UU远程 Windows 版客户端","operatingSystem":"Windows","softwareVersion":"4.48.3","dateModified":"2026-11-23"},
                      {"name":"网易UU远程 macOS 版客户端","operatingSystem":"macOS","softwareVersion":"4.49.0","dateModified":"2026-11-24"}]"#;
        assert_eq!(parse_windows_version(html).as_deref(), Some("4.48.3"));
        assert_eq!(parse_windows_version(r#"{"operatingSystem":"macOS","softwareVersion":"4.49.0"}"#), None);
    }

    #[test]
    fn never_slices_through_a_multibyte_character() {
        // The window cut used to land inside Chinese text and panic the refresh task.
        let html = format!("\"operatingSystem\":\"Windows\"{}{}", "中".repeat(200),
                           "\"softwareVersion\":\"4.48.3\"");
        assert_eq!(parse_windows_version(&html), None);
        assert_eq!(parse_windows_version("\"operatingSystem\":\"Windows\"中\""), None);
    }

    #[test]
    fn picks_the_highest_four_part_manifest_directory() {
        let json = r#"[{"name":"4.48.2.5754","type":"dir"},{"name":"4.48.10.1","type":"dir"},
                       {"name":"4.46.0.4716","type":"dir"},{"name":"README.md","type":"file"}]"#;
        assert_eq!(parse_winget_versions(json).as_deref(), Some("4.48.10.1"));
    }

    #[test]
    fn never_moves_backwards() {
        assert!(is_newer("4.48.4.6020", "4.45.0.4318"));
        assert!(is_newer("4.48.10.1", "4.48.4.6020"));
        assert!(is_newer("4.49.0.0", "4.48.10.1"));
        assert!(!is_newer("4.48.3.5946", "4.48.3.5946"));
        assert!(!is_newer("4.48.3.0", "4.48.3.5946"));
        assert!(!is_newer("4.48.1.5606", "4.48.4.6020"));
    }
}
