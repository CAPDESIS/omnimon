use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Safari,
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

fn map_cdp_targets_to_tabs(targets: Vec<CdpTabTarget>) -> Vec<BrowserTab> {
    targets
        .into_iter()
        .filter(|t| t.target_type.as_deref() == Some("page"))
        .map(|t| BrowserTab {
            id: t.id,
            title: t.title.unwrap_or_default(),
            url: t.url.unwrap_or_default(),
            browser: BrowserKind::Chrome,
        })
        .collect()
}

pub fn cdp_list_tabs(base_url: &str) -> Result<Vec<BrowserTab>, String> {
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

    Ok(map_cdp_targets_to_tabs(targets))
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

    fn list_chrome_tabs(&self) -> Result<Vec<BrowserTab>, String> {
        let script = r#"
on sanitizeText(inputText)
    set t to inputText as text
    return do shell script "printf %s " & quoted form of t & " | tr '\t\r\n' '   '"
end sanitizeText

tell application "Google Chrome"
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
"#;
        let out = Self::run_osascript(script, &[])?;
        Ok(Self::parse_lines(&out, BrowserKind::Chrome))
    }

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

    fn close_chrome_tab(&self, tab: &BrowserTab) -> Result<bool, String> {
        let script = r#"
on run argv
    set targetID to item 1 of argv
    set targetURL to item 2 of argv
    tell application "Google Chrome"
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

    fn close_safari_tab(&self, tab: &BrowserTab) -> Result<bool, String> {
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
            BrowserKind::Chrome => self.list_chrome_tabs(),
            BrowserKind::Safari => self.list_safari_tabs(),
        }
    }

    fn close_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
        match browser {
            BrowserKind::Chrome => self.close_chrome_tab(tab),
            BrowserKind::Safari => self.close_safari_tab(tab),
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub struct NativeTabProvider;

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl NativeTabProvider {
    const CDP_BASE: &'static str = "http://localhost:9222";

    fn cdp_list_tabs(&self) -> Result<Vec<BrowserTab>, String> {
        cdp_list_tabs(Self::CDP_BASE)
    }

    fn cdp_close_tab(&self, tab_id: &str) -> Result<bool, String> {
        cdp_close_tab(Self::CDP_BASE, tab_id)
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl TabProvider for NativeTabProvider {
    fn list_tabs(&self, browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
        match browser {
            BrowserKind::Chrome => self.cdp_list_tabs(),
            BrowserKind::Safari => Ok(Vec::new()),
        }
    }

    fn close_tab(&self, browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
        match browser {
            BrowserKind::Chrome => self.cdp_close_tab(&tab.id),
            BrowserKind::Safari => Ok(false),
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
