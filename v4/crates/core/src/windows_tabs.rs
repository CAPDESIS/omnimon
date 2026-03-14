//! Windows-native browser tab detection using UI Automation.
//!
//! Enumerates browser tabs without requiring Chrome DevTools Protocol (CDP)
//! or any special launch flags. Uses the Windows UI Automation API to walk
//! the accessibility tree of browser windows and extract tab titles.
//!
//! Fallback strategy:
//! 1. Try UI Automation to get all tabs from the tab strip
//! 2. Fall back to window titles (only gives active tab per window)

#![cfg(target_os = "windows")]

use crate::browser::{BrowserKind, BrowserTab};
use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationElementArray,
    TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_TabItemControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible,
};

/// Process name patterns for each browser.
fn browser_exe_names(browser: BrowserKind) -> &'static [&'static str] {
    match browser {
        BrowserKind::Chrome => &["chrome.exe"],
        BrowserKind::Brave => &["brave.exe"],
        BrowserKind::Edge => &["msedge.exe"],
        BrowserKind::Arc => &["arc.exe"],
        _ => &[],
    }
}

/// Window class names used by Chromium-based browsers.
const CHROMIUM_CLASS_NAME: &str = "Chrome_WidgetWin_1";

/// Get the executable name (lowercase) for a process ID.
fn process_exe_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let pwstr = PWSTR(buf.as_mut_ptr());
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, pwstr, &mut len).ok()?;
        let _ = windows::Win32::Foundation::CloseHandle(handle);
        let path = OsString::from_wide(&buf[..len as usize])
            .to_string_lossy()
            .to_string();
        path.rsplit('\\').next().map(|s| s.to_lowercase())
    }
}

/// Information about a browser window found via EnumWindows.
#[derive(Debug)]
struct BrowserWindow {
    hwnd: HWND,
    pid: u32,
    title: String,
    browser: BrowserKind,
}

/// Context passed to the EnumWindows callback.
struct EnumCtx {
    results: Vec<BrowserWindow>,
    cache: HashMap<u32, Option<String>>,
    exe_names: &'static [&'static str],
    browser: BrowserKind,
}

/// Enumerate all visible browser windows for the given browser kind.
fn enum_browser_windows(browser: BrowserKind) -> Vec<BrowserWindow> {
    let exe_names = browser_exe_names(browser);
    if exe_names.is_empty() {
        return Vec::new();
    }

    let mut ctx = EnumCtx {
        results: Vec::new(),
        cache: HashMap::new(),
        exe_names,
        browser,
    };

    unsafe {
        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut EnumCtx);

            if !IsWindowVisible(hwnd).as_bool() {
                return TRUE;
            }

            // Check window class
            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_buf);
            if class_len == 0 {
                return TRUE;
            }
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
            if class_name != CHROMIUM_CLASS_NAME {
                return TRUE;
            }

            // Get PID
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return TRUE;
            }

            // Check exe name (cached)
            let exe = ctx
                .cache
                .entry(pid)
                .or_insert_with(|| process_exe_name(pid))
                .clone();

            let matches = exe
                .as_ref()
                .map(|e| ctx.exe_names.iter().any(|n| *n == e.as_str()))
                .unwrap_or(false);

            if !matches {
                return TRUE;
            }

            // Get window title
            let title_len = GetWindowTextLengthW(hwnd);
            if title_len <= 0 {
                return TRUE;
            }
            let mut title_buf = vec![0u16; (title_len + 1) as usize];
            let actual = GetWindowTextW(hwnd, &mut title_buf);
            if actual <= 0 {
                return TRUE;
            }
            let title = String::from_utf16_lossy(&title_buf[..actual as usize]);

            if title.is_empty() {
                return TRUE;
            }

            ctx.results.push(BrowserWindow {
                hwnd,
                pid,
                title,
                browser: ctx.browser,
            });

            TRUE
        }

        let _ = EnumWindows(
            Some(callback),
            LPARAM(&mut ctx as *mut EnumCtx as isize),
        );
    }

    ctx.results
}

/// Strip the browser suffix from a window title to get the tab title.
/// Chrome format: "Tab Title - Google Chrome"
fn strip_browser_suffix(title: &str, browser: BrowserKind) -> String {
    let suffixes: &[&str] = match browser {
        BrowserKind::Chrome => &[" - Google Chrome", " \u{2014} Google Chrome"],
        BrowserKind::Brave => &[
            " - Brave",
            " \u{2014} Brave",
            " - Brave Browser",
            " \u{2014} Brave Browser",
        ],
        BrowserKind::Edge => &[
            " - Microsoft Edge",
            " \u{2014} Microsoft Edge",
            " - Microsoft\u{a0}Edge",
        ],
        BrowserKind::Arc => &[" - Arc", " \u{2014} Arc"],
        _ => &[],
    };

    for suffix in suffixes {
        if let Some(stripped) = title.strip_suffix(suffix) {
            return stripped.to_string();
        }
    }
    title.to_string()
}

/// Try to list all tabs using Windows UI Automation.
/// Accesses the tab strip control in browser windows and reads each tab item name.
fn uia_list_tabs(windows: &[BrowserWindow]) -> Result<Vec<BrowserTab>, String> {
    if windows.is_empty() {
        return Ok(Vec::new());
    }

    unsafe {
        // Initialize COM for this thread
        let com_init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let needs_uninit = com_init.is_ok();

        let result = (|| -> Result<Vec<BrowserTab>, String> {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                    .map_err(|e| format!("Failed to create UIAutomation: {e}"))?;

            let mut all_tabs = Vec::new();
            let mut tab_counter = 0u32;

            for window in windows {
                let element: IUIAutomationElement = match automation.ElementFromHandle(window.hwnd)
                {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Create condition: ControlType == TabItem
                let tab_item_id = UIA_TabItemControlTypeId.0;
                let variant = {
                    use windows::Win32::System::Variant::*;
                    let mut v = VARIANT::default();
                    // VT_I4 variant with the control type ID
                    (*v.Anonymous.Anonymous).vt = VT_I4;
                    (*v.Anonymous.Anonymous).Anonymous.lVal = tab_item_id as i32;
                    v
                };
                let condition = match automation.CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    variant,
                ) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Find all TabItem elements in the window tree
                let tab_items: Result<IUIAutomationElementArray, _> =
                    element.FindAll(TreeScope_Descendants, &condition);

                match tab_items {
                    Ok(items) => {
                        let count = items.Length().unwrap_or(0);
                        if count == 0 {
                            // No tab items found via UIA, use window title
                            tab_counter += 1;
                            let tab_title =
                                strip_browser_suffix(&window.title, window.browser);
                            if !tab_title.is_empty() {
                                all_tabs.push(BrowserTab {
                                    id: format!(
                                        "win-{}-{}-{}",
                                        window.browser.display_name().to_lowercase(),
                                        window.pid,
                                        tab_counter,
                                    ),
                                    title: tab_title,
                                    url: String::new(),
                                    browser: window.browser,
                                });
                            }
                            continue;
                        }
                        for i in 0..count {
                            if let Ok(item) = items.GetElement(i) {
                                let name = item
                                    .CurrentName()
                                    .map(|n| n.to_string())
                                    .unwrap_or_default();

                                if name.is_empty() {
                                    continue;
                                }

                                tab_counter += 1;
                                all_tabs.push(BrowserTab {
                                    id: format!(
                                        "uia-{}-{}-{}",
                                        window.browser.display_name().to_lowercase(),
                                        window.pid,
                                        tab_counter,
                                    ),
                                    title: name,
                                    url: String::new(),
                                    browser: window.browser,
                                });
                            }
                        }
                    }
                    Err(_) => {
                        // UIA enumeration failed for this window, use window title
                        tab_counter += 1;
                        let tab_title = strip_browser_suffix(&window.title, window.browser);
                        if !tab_title.is_empty() {
                            all_tabs.push(BrowserTab {
                                id: format!(
                                    "win-{}-{}-{}",
                                    window.browser.display_name().to_lowercase(),
                                    window.pid,
                                    tab_counter,
                                ),
                                title: tab_title,
                                url: String::new(),
                                browser: window.browser,
                            });
                        }
                    }
                }
            }

            Ok(all_tabs)
        })();

        if needs_uninit {
            CoUninitialize();
        }

        result
    }
}

/// Fallback: list tabs using just window titles (only active tab per window).
fn window_title_tabs(windows: &[BrowserWindow]) -> Vec<BrowserTab> {
    let mut tabs = Vec::new();
    for (i, w) in windows.iter().enumerate() {
        let tab_title = strip_browser_suffix(&w.title, w.browser);
        if tab_title.is_empty() || tab_title == w.browser.display_name() {
            continue;
        }
        tabs.push(BrowserTab {
            id: format!(
                "wt-{}-{}-{}",
                w.browser.display_name().to_lowercase(),
                w.pid,
                i,
            ),
            title: tab_title,
            url: String::new(),
            browser: w.browser,
        });
    }
    tabs
}

/// List browser tabs using Windows-native APIs.
/// Strategy: UIA first, window titles as fallback.
pub fn list_tabs_native(browser: BrowserKind) -> Vec<BrowserTab> {
    let windows = enum_browser_windows(browser);
    if windows.is_empty() {
        return Vec::new();
    }

    // Try UI Automation first for full tab list
    match uia_list_tabs(&windows) {
        Ok(tabs) if !tabs.is_empty() => return tabs,
        Ok(_) => {}
        Err(e) => {
            tracing::debug!(
                "UI Automation tab detection failed for {}: {}",
                browser.display_name(),
                e
            );
        }
    }

    // Fallback: window titles (only active tab per window)
    let wt_tabs = window_title_tabs(&windows);
    if !wt_tabs.is_empty() {
        return wt_tabs;
    }

    Vec::new()
}

/// Focus a browser window matching the tab's title.
pub fn focus_tab_native(browser: BrowserKind, tab: &BrowserTab) -> Result<bool, String> {
    let windows = enum_browser_windows(browser);
    for w in &windows {
        let active_title = strip_browser_suffix(&w.title, w.browser);
        if active_title == tab.title {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    SetForegroundWindow, ShowWindow, SW_RESTORE,
                };
                let _ = ShowWindow(w.hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(w.hwnd);
            }
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_chrome_suffix() {
        assert_eq!(
            strip_browser_suffix("GitHub - Google Chrome", BrowserKind::Chrome),
            "GitHub"
        );
        assert_eq!(
            strip_browser_suffix("Tab \u{2014} Google Chrome", BrowserKind::Chrome),
            "Tab"
        );
    }

    #[test]
    fn strip_edge_suffix() {
        assert_eq!(
            strip_browser_suffix("Bing - Microsoft Edge", BrowserKind::Edge),
            "Bing"
        );
    }

    #[test]
    fn strip_brave_suffix() {
        assert_eq!(
            strip_browser_suffix("DuckDuckGo - Brave", BrowserKind::Brave),
            "DuckDuckGo"
        );
    }

    #[test]
    fn no_suffix_returns_original() {
        assert_eq!(
            strip_browser_suffix("Just a title", BrowserKind::Chrome),
            "Just a title"
        );
    }

    /// Run with: cargo test -p core live_tab_detection -- --nocapture --ignored
    #[test]
    #[ignore]
    fn live_tab_detection() {
        let browsers = [
            BrowserKind::Chrome,
            BrowserKind::Edge,
            BrowserKind::Brave,
        ];
        for browser in &browsers {
            let tabs = list_tabs_native(*browser);
            println!("[{}] {} tabs detected:", browser.display_name(), tabs.len());
            for tab in &tabs {
                println!("  - [{}] {}", tab.id, tab.title);
            }
        }
    }

    #[test]
    fn browser_exe_names_are_correct() {
        assert_eq!(browser_exe_names(BrowserKind::Chrome), &["chrome.exe"]);
        assert_eq!(browser_exe_names(BrowserKind::Edge), &["msedge.exe"]);
        assert_eq!(browser_exe_names(BrowserKind::Brave), &["brave.exe"]);
        assert!(browser_exe_names(BrowserKind::Safari).is_empty());
        assert!(browser_exe_names(BrowserKind::Firefox).is_empty());
    }
}
