use serde::{Deserialize, Serialize};

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

#[cfg(target_os = "macos")]
pub struct NativeTabProvider;

#[cfg(target_os = "macos")]
impl NativeTabProvider {
    fn run_osascript(script: &str, args: &[&str]) -> Result<String, String> {
        use std::process::Command;

        let mut cmd = Command::new("osascript");
        cmd.arg("-");
        for arg in args {
            cmd.arg(arg);
        }
        let output = cmd
            .arg(script)
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
impl TabProvider for NativeTabProvider {
    fn list_tabs(&self, _browser: BrowserKind) -> Result<Vec<BrowserTab>, String> {
        Ok(Vec::new())
    }

    fn close_tab(&self, _browser: BrowserKind, _tab: &BrowserTab) -> Result<bool, String> {
        Ok(false)
    }
}
