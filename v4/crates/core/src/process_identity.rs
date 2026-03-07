use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcessGroupIdentity {
    pub key: String,
    pub identity_type: String,
    pub display_name: String,
    pub group: String,
}

pub fn detect_bundle_id(exe: Option<&Path>) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let exe = exe?;
        let path = exe.to_string_lossy();
        let marker = ".app/";
        let pos = path.find(marker)?;
        let bundle_root = &path[..pos + 4];
        Some(bundle_root.to_ascii_lowercase())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = exe;
        None
    }
}

pub fn normalize_process_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut depth = 0u32;
    let mut prev_space = false;

    for ch in name.chars() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            _ => {
                let mapped = if ch.is_ascii_alphanumeric() { ch } else { ' ' };
                if mapped == ' ' {
                    if !prev_space {
                        out.push(' ');
                        prev_space = true;
                    }
                } else {
                    out.push(mapped.to_ascii_lowercase());
                    prev_space = false;
                }
            }
        }
    }

    out.trim().to_string()
}

pub fn browser_family(name: &str, exec_name: &str, exe_path: Option<&str>) -> Option<&'static str> {
    let haystack = format!(
        "{} {} {}",
        name.to_ascii_lowercase(),
        exec_name.to_ascii_lowercase(),
        exe_path.unwrap_or_default().to_ascii_lowercase()
    );

    if haystack.contains("chrome") && !haystack.contains("chromium") {
        return Some("Chrome");
    }
    if haystack.contains("chromium") {
        return Some("Chromium");
    }
    if haystack.contains("safari") || haystack.contains("webkit.webcontent") {
        return Some("Safari");
    }
    if haystack.contains("brave") {
        return Some("Brave");
    }
    if haystack.contains("edge") || haystack.contains("msedge") {
        return Some("Edge");
    }
    if haystack.contains("arc") {
        return Some("Arc");
    }
    if haystack.contains("firefox") {
        return Some("Firefox");
    }
    None
}

pub fn classify_group(
    name: &str,
    exec_name: &str,
    exe_path: Option<&str>,
    is_system: bool,
) -> String {
    if browser_family(name, exec_name, exe_path).is_some() {
        return "Browser".to_string();
    }
    if is_system {
        return "System".to_string();
    }
    String::new()
}

pub fn resolve_group_identity(
    name: &str,
    exec_name: &str,
    exe_path: Option<&str>,
    bundle_id: Option<&str>,
    is_system: bool,
) -> ProcessGroupIdentity {
    if let Some(browser) = browser_family(name, exec_name, exe_path) {
        return ProcessGroupIdentity {
            key: format!("browser:{}", browser.to_ascii_lowercase()),
            identity_type: "browser_family".to_string(),
            display_name: browser.to_string(),
            group: "Browser".to_string(),
        };
    }

    if let Some(bundle) = bundle_id {
        let display_name = Path::new(bundle)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| name.to_string());
        return ProcessGroupIdentity {
            key: format!("bundle:{}", bundle.to_ascii_lowercase()),
            identity_type: "bundle_id".to_string(),
            display_name,
            group: classify_group(name, exec_name, exe_path, is_system),
        };
    }

    if let Some(path) = exe_path {
        let display_name = Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| name.to_string());
        let basename = Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| normalize_process_name(name));
        return ProcessGroupIdentity {
            key: format!("exec:{}", basename),
            identity_type: "exec_name".to_string(),
            display_name,
            group: classify_group(name, exec_name, exe_path, is_system),
        };
    }

    let normalized = normalize_process_name(name);
    ProcessGroupIdentity {
        key: format!("name:{}", normalized),
        identity_type: "normalized_name".to_string(),
        display_name: name.to_string(),
        group: classify_group(name, exec_name, exe_path, is_system),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_removes_renderer_suffix_noise() {
        assert_eq!(
            normalize_process_name("Google Chrome Helper (Renderer)"),
            "google chrome helper"
        );
    }

    #[test]
    fn browser_identity_groups_helpers_together() {
        let identity = resolve_group_identity(
            "Google Chrome Helper (Renderer)",
            "Google Chrome Helper",
            Some("/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper"),
            Some("/applications/google chrome.app"),
            false,
        );
        assert_eq!(identity.key, "browser:chrome");
        assert_eq!(identity.display_name, "Chrome");
    }
}
