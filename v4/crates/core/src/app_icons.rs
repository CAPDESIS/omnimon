use base64::Engine;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

static ICON_CACHE: OnceLock<RwLock<HashMap<String, Option<String>>>> = OnceLock::new();

fn icon_cache() -> &'static RwLock<HashMap<String, Option<String>>> {
    ICON_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn resolve_process_icon_data_url(
    exe_path: Option<&str>,
    bundle_id: Option<&str>,
    process_name: &str,
    exec_name: &str,
) -> Option<String> {
    let cache_key = bundle_id
        .map(|value| format!("bundle:{value}"))
        .or_else(|| exe_path.map(|value| format!("path:{value}")))
        .unwrap_or_else(|| format!("name:{}:{}", process_name, exec_name));

    if let Ok(cache) = icon_cache().read() {
        if let Some(cached) = cache.get(&cache_key) {
            return cached.clone();
        }
    }

    let resolved = resolve_native_icon_data_url(exe_path, bundle_id, process_name, exec_name);

    if let Ok(mut cache) = icon_cache().write() {
        // Cap cache at 2048 entries to prevent unbounded memory growth
        // in long-running sessions with high process churn.
        if cache.len() >= 2048 {
            cache.clear();
        }
        cache.insert(cache_key, resolved.clone());
    }

    resolved
}

fn resolve_native_icon_data_url(
    exe_path: Option<&str>,
    bundle_id: Option<&str>,
    process_name: &str,
    exec_name: &str,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(data) = macos_icon_data_url(exe_path, bundle_id) {
            return Some(data);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(data) = linux_icon_data_url(exe_path, process_name, exec_name) {
            return Some(data);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(data) = windows_icon_data_url(exe_path) {
            return Some(data);
        }
    }

    let _ = (exe_path, bundle_id, process_name, exec_name);
    None
}

fn file_to_data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        _ => return None,
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

#[cfg(target_os = "macos")]
fn macos_icon_data_url(exe_path: Option<&str>, bundle_id: Option<&str>) -> Option<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::process::Command;

    let app_root = bundle_id
        .map(PathBuf::from)
        .or_else(|| exe_path.and_then(app_bundle_root_from_exe))?;

    let icon_source = macos_bundle_icon_file(&app_root)
        .or_else(|| first_matching_file(&app_root.join("Contents/Resources"), &["icns", "png"]))?;

    if icon_source.extension().and_then(|ext| ext.to_str()) == Some("png") {
        return file_to_data_url(&icon_source);
    }

    let mut hasher = DefaultHasher::new();
    icon_source.hash(&mut hasher);
    let output_path = std::env::temp_dir().join(format!("omnimon-icon-{}.png", hasher.finish()));

    if !output_path.exists() {
        let status = Command::new("sips")
            .args([
                "-s",
                "format",
                "png",
                &icon_source.to_string_lossy(),
                "--out",
                &output_path.to_string_lossy(),
            ])
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
    }

    file_to_data_url(&output_path)
}

#[cfg(target_os = "macos")]
fn app_bundle_root_from_exe(exe_path: &str) -> Option<PathBuf> {
    let marker = ".app/";
    let pos = exe_path.find(marker)?;
    Some(PathBuf::from(&exe_path[..pos + 4]))
}

#[cfg(target_os = "macos")]
fn macos_bundle_icon_file(app_root: &Path) -> Option<PathBuf> {
    let plist = fs::read_to_string(app_root.join("Contents/Info.plist")).ok()?;
    let icon_name = plist_value(&plist, "CFBundleIconFile")?;
    let mut path = app_root.join("Contents/Resources").join(&icon_name);
    if path.extension().is_none() {
        path.set_extension("icns");
    }
    path.exists().then_some(path)
}

#[cfg(target_os = "macos")]
fn plist_value(contents: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{key}</key>");
    let start = contents.find(&needle)?;
    let tail = &contents[start + needle.len()..];
    let string_open = tail.find("<string>")? + "<string>".len();
    let string_close = tail[string_open..].find("</string>")?;
    Some(
        tail[string_open..string_open + string_close]
            .trim()
            .to_string(),
    )
}

#[cfg(target_os = "linux")]
fn linux_icon_data_url(
    exe_path: Option<&str>,
    process_name: &str,
    exec_name: &str,
) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(path) = exe_path {
        if let Some(stem) = Path::new(path).file_stem().and_then(|stem| stem.to_str()) {
            candidates.push(stem.to_string());
        }
    }
    candidates.push(exec_name.to_string());
    candidates.push(process_name.to_string());
    candidates.push(crate::process_identity::normalize_process_name(
        process_name,
    ));

    let dirs = [
        "/usr/share/icons/hicolor/256x256/apps",
        "/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/64x64/apps",
        "/usr/share/pixmaps",
        "/var/lib/flatpak/exports/share/icons/hicolor/256x256/apps",
    ];

    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }
        let variants = [candidate.clone(), candidate.to_ascii_lowercase()];
        for dir in dirs {
            for variant in &variants {
                for ext in ["png", "svg", "jpg", "jpeg"] {
                    let path = Path::new(dir).join(format!("{variant}.{ext}"));
                    if path.exists() {
                        if let Some(data) = file_to_data_url(&path) {
                            return Some(data);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn windows_icon_data_url(exe_path: Option<&str>) -> Option<String> {
    use std::process::Command;

    let exe_path = exe_path?;
    let output = std::env::temp_dir().join(format!(
        "omnimon-win-icon-{}.png",
        exe_path.replace(['\\', ':'], "_")
    ));

    if !output.exists() {
        let command = format!(
            "$src = '{}'; $dst = '{}'; Add-Type -AssemblyName System.Drawing; $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($src); if ($icon -ne $null) {{ $icon.ToBitmap().Save($dst, [System.Drawing.Imaging.ImageFormat]::Png) }}",
            exe_path.replace('\'', "''"),
            output.display().to_string().replace('\'', "''")
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &command])
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
    }

    file_to_data_url(&output)
}

#[cfg(target_os = "macos")]
fn first_matching_file(dir: &Path, extensions: &[&str]) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|ext| ext.to_str())?;
        if extensions
            .iter()
            .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        {
            return Some(path);
        }
    }
    None
}
