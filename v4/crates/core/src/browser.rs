use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Validate a tab ID: reject empty, >512 chars, control chars, path traversal chars.
pub fn sanitize_tab_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Tab ID must not be empty".to_string());
    }
    if id.len() > 512 {
        return Err("Tab ID exceeds maximum length of 512".to_string());
    }
    if id.chars().any(|c| c.is_control()) {
        return Err("Tab ID contains control characters".to_string());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("Tab ID contains path traversal characters".to_string());
    }
    Ok(())
}

/// Validate a tab URL: reject >4096 chars, control chars, disallowed schemes.
pub fn sanitize_tab_url(url: &str) -> Result<(), String> {
    if url.len() > 4096 {
        return Err("Tab URL exceeds maximum length of 4096".to_string());
    }
    if url.chars().any(|c| c.is_control()) {
        return Err("Tab URL contains control characters".to_string());
    }
    if !url.is_empty() {
        let allowed_prefixes = [
            "http://", "https://", "about:", "chrome://", "chrome-extension://",
            "safari-web-extension://", "brave://", "edge://", "arc://",
        ];
        if !allowed_prefixes.iter().any(|p| url.starts_with(p)) {
            return Err(format!("Tab URL has disallowed scheme: {}", url.split(':').next().unwrap_or("")));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Safari,
    Brave,
    Edge,
    Arc,
    Firefox,
}

impl BrowserKind {
    /// Whether this browser supports CDP (Chrome DevTools Protocol).
    pub fn supports_cdp(&self) -> bool {
        matches!(self, BrowserKind::Chrome | BrowserKind::Brave | BrowserKind::Edge | BrowserKind::Arc)
    }

    /// macOS application name for AppleScript.
    pub fn applescript_app_name(&self) -> Option<&'static str> {
        match self {
            BrowserKind::Chrome => Some("Google Chrome"),
            BrowserKind::Safari => Some("Safari"),
            BrowserKind::Brave => Some("Brave Browser"),
            BrowserKind::Edge => Some("Microsoft Edge"),
            BrowserKind::Arc => Some("Arc"),
            BrowserKind::Firefox => None, // Firefox doesn't support AppleScript tab enumeration
        }
    }

    /// Default CDP debugging port.
    pub fn cdp_port(&self) -> u16 {
        match self {
            BrowserKind::Chrome => 9222,
            BrowserKind::Brave => 9223,
            BrowserKind::Edge => 9224,
            BrowserKind::Arc => 9225,
            _ => 0,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            BrowserKind::Chrome => "Chrome",
            BrowserKind::Safari => "Safari",
            BrowserKind::Brave => "Brave",
            BrowserKind::Edge => "Edge",
            BrowserKind::Arc => "Arc",
            BrowserKind::Firefox => "Firefox",
        }
    }

    /// All browser kinds.
    pub fn all() -> &'static [BrowserKind] {
        &[
            BrowserKind::Chrome,
            BrowserKind::Safari,
            BrowserKind::Brave,
            BrowserKind::Edge,
            BrowserKind::Arc,
        ]
    }

    /// Parse from a string (e.g. IPC input).
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Chrome" => Ok(BrowserKind::Chrome),
            "Safari" => Ok(BrowserKind::Safari),
            "Brave" => Ok(BrowserKind::Brave),
            "Edge" => Ok(BrowserKind::Edge),
            "Arc" => Ok(BrowserKind::Arc),
            "Firefox" => Ok(BrowserKind::Firefox),
            _ => Err(format!("Unknown browser: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub browser: BrowserKind,
}

pub trait TabProvider {
    fn list_tabs(&self, browser: BrowserKind) -> Result<Vec<BrowserTab>, String>;
    fn close_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String>;
}

#[derive(Debug, Deserialize)]
struct CdpTabTarget {
    id: String,
    title: Option<String>,
    url: Option<String>,
    #[serde(rename = "type")]
    target_type: Option<String>,
}

fn build_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to build tokio runtime: {e}"))
}

fn map_cdp_targets_to_tabs(targets: Vec<CdpTabTarget>, browser: BrowserKind) -> Vec<BrowserTab> {
    targets
        .into_iter()
        .filter(|t| t.target_type.as_deref() == Some("page"))
        .map(|t| BrowserTab {
            id: t.id,
            title: t.title.unwrap_or_default(),
            url: t.url.unwrap_or_default(),
            browser,
        })
        .collect()
}

pub fn cdp_list_tabs(base_url: &str) -> Result<Vec<BrowserTab>, String> {
    cdp_list_tabs_for(base_url, BrowserKind::Chrome)
}

pub fn cdp_list_tabs_for(base_url: &str, browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
    let runtime = build_runtime()?;
    let targets_result = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let response = client.get(format!("{}/json/list", base_url)).send().await?;

        if !response.status().is_success() {
            return Ok::<Vec<CdpTabTarget>, reqwest::Error>(Vec::new());
        }

        let parsed = response.json::<Vec<CdpTabTarget>>().await?;
        Ok::<Vec<CdpTabTarget>, reqwest::Error>(parsed)
    });

    let targets = match targets_result {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    Ok(map_cdp_targets_to_tabs(targets, browser))
}

pub fn cdp_close_tab(base_url: &str, tab_id: &str) -> Result<bool, String> {
    if tab_id.trim().is_empty() {
        return Ok(false);
    }

    if tab_id.contains('/') || tab_id.contains('\\') || tab_id.contains('?') || tab_id.contains('#') {
        return Err("Invalid tab ID".to_string());
    }

    let runtime = build_runtime()?;
    let close_result = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let endpoint = format!("{}/json/close/{}", base_url, tab_id);
        let response = client.get(endpoint).send().await?;
        Ok::<bool, reqwest::Error>(response.status().is_success())
    });

    match close_result {
        Ok(closed) => Ok(closed),
        Err(_) => Ok(false),
    }
}

#[cfg(target_os = "macos")]
pub struct NativeTabProvider;

#[cfg(target_os = "macos")]
impl NativeTabProvider {
    fn run_osascript(script: &str, args: &[&str]) -> Result<String, String> {
        use std::process::Command;

        let mut cmd = Command::new("osascript");
        cmd.arg("-e");
        cmd.arg(script);
        for arg in args {
            cmd.arg(arg);
        }
        let output = cmd
            .output()
            .map_err(|e| format!("osascript execution failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "osascript returned non-zero status".to_string()
            } else {
                stderr
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn parse_lines(raw: &str, browser: BrowserKind) -> Vec<BrowserTab> {
        raw.lines()
            .filter_map(|line| {
                if line.trim().is_empty() {
                    return None;
                }
                let mut parts = line.split('');
                let id = parts.next()?.to_string();
                let title = parts.next().unwrap_or_default().to_string();
                let url = parts.next().unwrap_or_default().to_string();
                Some(BrowserTab {
                    id,
                    title,
                    url,
                    browser,
                })
            })
            .collect()
    }

    /// Generic Chromium tab listing (Chrome, Brave, Edge, Arc all use `title of t`).
    fn list_chromium_tabs(&self, browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
        let app_name = browser.applescript_app_name()
            .ok_or_else(|| format!("{} does not support AppleScript", browser.display_name()))?;
        let script = format!(
            r#"
on sanitizeText(inputText)
    set t to inputText as text
    return do shell script "printf %s " & quoted form of t & " | tr '\t\r\n' '   '"
end sanitizeText

tell application "{app}"
    set sep to (character id 31)
    set output to ""
    repeat with w in windows
        repeat with t in tabs of w
            set tabID to (id of t as text)
            set tabTitle to my sanitizeText(title of t as text)
            set tabURL to my sanitizeText(URL of t as text)
            set output to output & tabID & sep & tabTitle & sep & tabURL & linefeed
        end repeat
    end repeat
    return output
end tell
"#,
            app = app_name
        );
        let out = Self::run_osascript(&script, &[])?;
        Ok(Self::parse_lines(&out, browser))
    }

    /// Safari uses `name of t` instead of `title of t`.
    fn list_safari_tabs(&self) -> Result<Vec<BrowserTab>, String> {
        let script = r#"
on sanitizeText(inputText)
    set t to inputText as text
    return do shell script "printf %s " & quoted form of t & " | tr '\t\r\n' '   '"
end sanitizeText

tell application "Safari"
    set sep to (character id 31)
    set output to ""
    repeat with w in windows
        repeat with t in tabs of w
            set tabID to (id of t as text)
            set tabTitle to my sanitizeText(name of t as text)
            set tabURL to my sanitizeText(URL of t as text)
            set output to output & tabID & sep & tabTitle & sep & tabURL & linefeed
        end repeat
    end repeat
    return output
end tell
"#;
        let out = Self::run_osascript(script, &[])?;
        Ok(Self::parse_lines(&out, BrowserKind::Safari))
    }

    /// Generic Chromium close tab (works for Chrome, Brave, Edge, Arc).
    fn close_chromium_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
        sanitize_tab_id(&tab.id)?;
        sanitize_tab_url(&tab.url)?;
        let app_name = browser.applescript_app_name()
            .ok_or_else(|| format!("{} does not support AppleScript", browser.display_name()))?;
        let script = format!(
            r#"
on run argv
    set targetID to item 1 of argv
    set targetURL to item 2 of argv
    tell application "{app}"
        repeat with w in windows
            repeat with t in tabs of w
                if ((id of t as text) is targetID) or ((URL of t as text) is targetURL) then
                    close t
                    return "closed"
                end if
            end repeat
        end repeat
    end tell
    return "not_found"
end run
"#,
            app = app_name
        );
        let out = Self::run_osascript(&script, &[&tab.id, &tab.url])?;
        Ok(out.trim() == "closed")
    }

    fn close_safari_tab(&self, tab: &BrowserTab) -> Result<bool, String> {
        sanitize_tab_id(&tab.id)?;
        sanitize_tab_url(&tab.url)?;
        let script = r#"
on run argv
    set targetID to item 1 of argv
    set targetURL to item 2 of argv
    tell application "Safari"
        repeat with w in windows
            repeat with t in tabs of w
                if ((id of t as text) is targetID) or ((URL of t as text) is targetURL) then
                    close t
                    return "closed"
                end if
            end repeat
        end repeat
    end tell
    return "not_found"
end run
"#;
        let out = Self::run_osascript(script, &[&tab.id, &tab.url])?;
        Ok(out.trim() == "closed")
    }
}

#[cfg(target_os = "macos")]
impl TabProvider for NativeTabProvider {
    fn list_tabs(&self, browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
        match browser {
            BrowserKind::Safari => self.list_safari_tabs(),
            BrowserKind::Chrome | BrowserKind::Brave | BrowserKind::Edge | BrowserKind::Arc => {
                self.list_chromium_tabs(browser)
            }
            BrowserKind::Firefox => Ok(Vec::new()), // not supported via AppleScript
        }
    }

    fn close_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
        match browser {
            BrowserKind::Safari => self.close_safari_tab(tab),
            BrowserKind::Chrome | BrowserKind::Brave | BrowserKind::Edge | BrowserKind::Arc => {
                self.close_chromium_tab(browser, tab)
            }
            BrowserKind::Firefox => Ok(false),
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub struct NativeTabProvider;

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl TabProvider for NativeTabProvider {
    fn list_tabs(&self, browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
        if browser.supports_cdp() {
            let port = browser.cdp_port();
            let base = format!("http://localhost:{}", port);
            cdp_list_tabs_for(&base, browser)
        } else {
            Ok(Vec::new())
        }
    }

    fn close_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
        if browser.supports_cdp() {
            let port = browser.cdp_port();
            let base = format!("http://localhost:{}", port);
            cdp_close_tab(&base, &tab.id)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn cdp_list_tabs_maps_and_filters_page_targets() {
        let mut server = Server::new();
        let _mock = server
            .mock("GET", "/json/list")
            .with_status(200)
            .with_body(
                r#"[
                    {"id":"a1","type":"page","title":"Tab A","url":"https://a.test"},
                    {"id":"worker1","type":"service_worker","title":"SW","url":""},
                    {"id":"a2","type":"page","title":"Tab B","url":"https://b.test"}
                ]"#,
            )
            .create();

        let tabs = cdp_list_tabs(&server.url()).expect("cdp list should not fail");
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, "a1");
        assert_eq!(tabs[1].id, "a2");
        assert_eq!(tabs[0].browser, BrowserKind::Chrome);
    }

    #[test]
    fn cdp_list_tabs_returns_empty_on_connection_error() {
        let tabs = cdp_list_tabs("http://127.0.0.1:9").expect("connection failures map to empty");
        assert!(tabs.is_empty());
    }

    #[test]
    fn cdp_list_tabs_returns_empty_on_non_success() {
        let mut server = Server::new();
        let _mock = server
            .mock("GET", "/json/list")
            .with_status(500)
            .with_body("oops")
            .create();

        let tabs = cdp_list_tabs(&server.url()).expect("non-success should be empty list");
        assert!(tabs.is_empty());
    }

    #[test]
    fn cdp_close_tab_handles_success_and_failure_paths() {
        let mut server = Server::new();
        let _close_ok = server
            .mock("GET", "/json/close/tab-ok")
            .with_status(200)
            .create();
        let _close_fail = server
            .mock("GET", "/json/close/tab-missing")
            .with_status(404)
            .create();

        let ok = cdp_close_tab(&server.url(), "tab-ok").expect("close should not error");
        let missing = cdp_close_tab(&server.url(), "tab-missing").expect("close should not error");
        let conn_refused =
            cdp_close_tab("http://127.0.0.1:9", "tab-any").expect("refused should map false");

        assert!(ok);
        assert!(!missing);
        assert!(!conn_refused);
    }

    #[test]
    fn cdp_close_tab_rejects_empty_tab_id() {
        let closed = cdp_close_tab("http://127.0.0.1:9", "").expect("empty id returns false");
        assert!(!closed);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_lines_extracts_tab_fields_for_macos() {
        let raw =
            "1\u{1f}Tab One\u{1f}https://example.com\n2\u{1f}Tab Two\u{1f}https://example.org\n";
        let tabs = NativeTabProvider::parse_lines(raw, BrowserKind::Chrome);
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, "1");
        assert_eq!(tabs[0].title, "Tab One");
        assert_eq!(tabs[0].url, "https://example.com");
    }

    #[test]
    fn sanitize_tab_id_rejects_empty() {
        assert!(sanitize_tab_id("").is_err());
    }

    #[test]
    fn sanitize_tab_id_rejects_long_input() {
        let long = "a".repeat(513);
        assert!(sanitize_tab_id(&long).is_err());
        assert!(sanitize_tab_id(&"a".repeat(512)).is_ok());
    }

    #[test]
    fn sanitize_tab_id_rejects_control_chars() {
        assert!(sanitize_tab_id("tab\x00id").is_err());
        assert!(sanitize_tab_id("tab\nid").is_err());
    }

    #[test]
    fn sanitize_tab_id_rejects_path_traversal() {
        assert!(sanitize_tab_id("../etc/passwd").is_err());
        assert!(sanitize_tab_id("foo/bar").is_err());
        assert!(sanitize_tab_id("foo\\bar").is_err());
    }

    #[test]
    fn sanitize_tab_id_accepts_valid() {
        assert!(sanitize_tab_id("abc-123").is_ok());
        assert!(sanitize_tab_id("F8B3A4D2-1234").is_ok());
    }

    #[test]
    fn sanitize_tab_url_rejects_long_input() {
        let long = format!("https://example.com/{}", "a".repeat(4097));
        assert!(sanitize_tab_url(&long).is_err());
    }

    #[test]
    fn sanitize_tab_url_rejects_control_chars() {
        assert!(sanitize_tab_url("https://example.com/\x00").is_err());
    }

    #[test]
    fn sanitize_tab_url_rejects_disallowed_schemes() {
        assert!(sanitize_tab_url("file:///etc/passwd").is_err());
        assert!(sanitize_tab_url("javascript:alert(1)").is_err());
        assert!(sanitize_tab_url("data:text/html,<h1>hi</h1>").is_err());
    }

    #[test]
    fn sanitize_tab_url_accepts_valid_schemes() {
        assert!(sanitize_tab_url("https://example.com").is_ok());
        assert!(sanitize_tab_url("http://localhost:3000").is_ok());
        assert!(sanitize_tab_url("about:blank").is_ok());
        assert!(sanitize_tab_url("chrome://settings").is_ok());
        assert!(sanitize_tab_url("").is_ok()); // empty is allowed (some tabs have no URL)
    }

    #[test]
    fn browser_kind_methods() {
        assert!(BrowserKind::Chrome.supports_cdp());
        assert!(BrowserKind::Brave.supports_cdp());
        assert!(BrowserKind::Edge.supports_cdp());
        assert!(BrowserKind::Arc.supports_cdp());
        assert!(!BrowserKind::Safari.supports_cdp());
        assert!(!BrowserKind::Firefox.supports_cdp());

        assert_eq!(BrowserKind::Chrome.cdp_port(), 9222);
        assert_eq!(BrowserKind::Brave.cdp_port(), 9223);
        assert_eq!(BrowserKind::Edge.cdp_port(), 9224);
        assert_eq!(BrowserKind::Arc.cdp_port(), 9225);

        assert_eq!(BrowserKind::Chrome.display_name(), "Chrome");
        assert_eq!(BrowserKind::Brave.display_name(), "Brave");

        assert!(BrowserKind::Chrome.applescript_app_name().is_some());
        assert!(BrowserKind::Firefox.applescript_app_name().is_none());
    }

    #[test]
    fn browser_kind_from_str_works() {
        assert_eq!(BrowserKind::from_str("Chrome").unwrap(), BrowserKind::Chrome);
        assert_eq!(BrowserKind::from_str("Brave").unwrap(), BrowserKind::Brave);
        assert_eq!(BrowserKind::from_str("Edge").unwrap(), BrowserKind::Edge);
        assert_eq!(BrowserKind::from_str("Arc").unwrap(), BrowserKind::Arc);
        assert!(BrowserKind::from_str("Unknown").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_provider_osascript_calls_are_non_panicking() {
        let provider = NativeTabProvider;
        let _ = provider.list_tabs(BrowserKind::Chrome);
        let _ = provider.list_tabs(BrowserKind::Safari);

        let dummy_tab = BrowserTab {
            id: "non-existent-id".to_string(),
            title: "Dummy".to_string(),
            url: "https://example.invalid".to_string(),
            browser: BrowserKind::Chrome,
        };
        let _ = provider.close_tab(BrowserKind::Chrome, &dummy_tab);
        let _ = provider.close_tab(BrowserKind::Safari, &dummy_tab);
    }
}
