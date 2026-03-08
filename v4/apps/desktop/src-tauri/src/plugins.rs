use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, VmState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const MAX_PLUGINS: usize = 32;
const MAX_SCRIPT_BYTES: usize = 256 * 1024;
const MAX_SCRIPT_MEMORY_BYTES: usize = 1024 * 1024;
const MAX_METRICS_PER_PLUGIN: usize = 64;
const MAX_TAGS_PER_METRIC: usize = 12;
const MAX_TEXT_FIELD_BYTES: usize = 120;
const PLUGIN_LOOP_SECS: u64 = 4;
const PLUGIN_TIMEOUT_MS: u64 = 150;
const PLUGIN_HOOK_GRANULARITY: u32 = 10_000;
const MANIFEST_FILENAME: &str = "index.json";

static ENGINE: OnceLock<Arc<PluginEngine>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginManifest {
    #[serde(default = "default_manifest_schema")]
    schema_version: u32,
    #[serde(default)]
    plugins: Vec<StoredPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPlugin {
    id: String,
    name: String,
    file_name: String,
    script_path: String,
    enabled: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginRunState {
    status: String,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    last_run_ms: Option<u128>,
    #[serde(default)]
    last_duration_ms: Option<u64>,
    #[serde(default)]
    metrics: Vec<PluginMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EngineState {
    plugins: Vec<StoredPlugin>,
    runtime: HashMap<String, PluginRunState>,
}

#[derive(Debug)]
struct PluginEngine {
    scripts_dir: PathBuf,
    manifest_path: PathBuf,
    state: Arc<RwLock<EngineState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub last_run_ms: Option<u128>,
    pub last_duration_ms: Option<u64>,
    pub metrics: Vec<PluginMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetric {
    pub name: String,
    pub label: String,
    pub kind: String,
    pub value: f64,
    pub unit: Option<String>,
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct PluginMetadataInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PluginOutput {
    #[serde(default)]
    metrics: Vec<PluginMetricInput>,
}

#[derive(Debug, Deserialize)]
struct PluginMetricInput {
    name: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    value: f64,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
struct PluginContext {
    timestamp_ms: u128,
    cpu_usage_percent: f32,
    total_memory_bytes: u64,
    used_memory_bytes: u64,
    free_memory_bytes: u64,
    swap_used_mb: u64,
    net_rx_bytes_per_sec: u64,
    net_tx_bytes_per_sec: u64,
    process_count: usize,
    top_processes: Vec<PluginProcessContext>,
}

#[derive(Debug, Clone, Serialize)]
struct PluginProcessContext {
    pid: u32,
    name: String,
    exec_name: String,
    cpu_pct: f32,
    memory_mb: f64,
    net_rx_bytes_per_sec: u64,
    net_tx_bytes_per_sec: u64,
}

#[derive(Debug)]
struct ValidationResult {
    name: String,
    description: Option<String>,
    version: Option<String>,
    metrics: Vec<PluginMetric>,
}

fn default_manifest_schema() -> u32 {
    1
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn plugin_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("plugin data dir unavailable: {err}"))?
        .join("plugins");
    fs::create_dir_all(root.join("scripts")).map_err(|err| err.to_string())?;
    Ok(root)
}

fn load_manifest(path: &Path) -> PluginManifest {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str::<PluginManifest>(&contents).unwrap_or_default(),
        Err(_) => PluginManifest::default(),
    }
}

fn save_manifest(path: &Path, plugins: &[StoredPlugin]) -> Result<(), String> {
    let manifest = PluginManifest {
        schema_version: default_manifest_schema(),
        plugins: plugins.to_vec(),
    };
    let contents = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(path, contents).map_err(|err| err.to_string())
}

fn humanize_plugin_id(id: &str) -> String {
    id.split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_file_stem(file_name: &str) -> Result<String, String> {
    let path = Path::new(file_name);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "plugins must use the .lua extension".to_string())?;
    if !extension.eq_ignore_ascii_case("lua") {
        return Err("plugins must use the .lua extension".to_string());
    }

    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid plugin file name".to_string())?;

    let mut sanitized = String::with_capacity(stem.len());
    let mut last_dash = false;
    for ch in stem.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if matches!(ch, '-' | '_' | ' ') {
            Some('-')
        } else {
            None
        };

        match normalized {
            Some('-') => {
                if !last_dash {
                    sanitized.push('-');
                    last_dash = true;
                }
            }
            Some(value) => {
                sanitized.push(value);
                last_dash = false;
            }
            None => {}
        }
    }

    let trimmed = sanitized.trim_matches('-').to_string();
    if trimmed.is_empty() {
        return Err("plugin file name must contain ASCII letters or numbers".to_string());
    }
    Ok(trimmed)
}

fn truncate_string(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut result = String::new();
        for ch in trimmed.chars().take(MAX_TEXT_FIELD_BYTES) {
            if ch.is_ascii_graphic() || ch == ' ' {
                result.push(ch);
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    })
}

fn build_context() -> PluginContext {
    let snapshot = macmon_core::watcher::get_cached_state();
    let mut top_processes = snapshot.cached_process_info;
    top_processes.sort_by(|left, right| right.memory_bytes.cmp(&left.memory_bytes));
    top_processes.truncate(24);

    PluginContext {
        timestamp_ms: now_unix_ms(),
        cpu_usage_percent: snapshot.cpu_usage_percent,
        total_memory_bytes: snapshot.total_memory_bytes,
        used_memory_bytes: snapshot.used_memory_bytes,
        free_memory_bytes: snapshot.free_memory_bytes,
        swap_used_mb: snapshot.swap_used_mb,
        net_rx_bytes_per_sec: snapshot.net_rx_bytes_per_sec,
        net_tx_bytes_per_sec: snapshot.net_tx_bytes_per_sec,
        process_count: top_processes.len(),
        top_processes: top_processes
            .into_iter()
            .map(|process| PluginProcessContext {
                pid: process.pid,
                name: process.name,
                exec_name: process.exec_name,
                cpu_pct: process.cpu_pct,
                memory_mb: (process.memory_bytes as f64 / 1_048_576.0 * 10.0).round() / 10.0,
                net_rx_bytes_per_sec: process.net_rx_bytes_per_sec,
                net_tx_bytes_per_sec: process.net_tx_bytes_per_sec,
            })
            .collect(),
    }
}

fn validate_metrics(metrics: Vec<PluginMetricInput>) -> Result<Vec<PluginMetric>, String> {
    if metrics.len() > MAX_METRICS_PER_PLUGIN {
        return Err(format!(
            "plugin returned too many metrics (max {MAX_METRICS_PER_PLUGIN})"
        ));
    }

    metrics
        .into_iter()
        .map(|metric| {
            if !metric.value.is_finite() {
                return Err("plugin metric values must be finite numbers".to_string());
            }

            if metric.tags.len() > MAX_TAGS_PER_METRIC {
                return Err(format!(
                    "metric '{}' has too many tags (max {MAX_TAGS_PER_METRIC})",
                    metric.name
                ));
            }

            let name = truncate_string(Some(metric.name.clone()))
                .ok_or_else(|| "metric name cannot be empty".to_string())?;
            let label = truncate_string(metric.label.clone()).unwrap_or_else(|| name.clone());
            let kind = truncate_string(metric.kind.clone())
                .unwrap_or_else(|| "gauge".to_string())
                .to_ascii_lowercase();

            if !matches!(kind.as_str(), "gauge" | "counter") {
                return Err(format!("metric '{name}' has unsupported kind '{kind}'"));
            }

            let mut tags = BTreeMap::new();
            for (key, value) in metric.tags {
                let safe_key = truncate_string(Some(key))
                    .ok_or_else(|| format!("metric '{name}' contains an empty tag key"))?;
                let safe_value = truncate_string(Some(value))
                    .ok_or_else(|| format!("metric '{name}' contains an empty tag value"))?;
                tags.insert(safe_key, safe_value);
            }

            Ok(PluginMetric {
                name,
                label,
                kind,
                value: (metric.value * 100.0).round() / 100.0,
                unit: truncate_string(metric.unit),
                tags,
            })
        })
        .collect()
}

fn run_plugin_source(file_name: &str, source: &str) -> Result<ValidationResult, String> {
    let lua = Lua::new();
    let _ = lua.set_memory_limit(MAX_SCRIPT_MEMORY_BYTES);

    let start = Instant::now();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(PLUGIN_HOOK_GRANULARITY),
        move |_, _| {
            if start.elapsed() > Duration::from_millis(PLUGIN_TIMEOUT_MS) {
                return Err(mlua::Error::RuntimeError(
                    "plugin execution exceeded the time budget".to_string(),
                ));
            }
            Ok(VmState::Continue)
        },
    )
    .map_err(|err| err.to_string())?;

    lua.load(source)
        .set_name(file_name)
        .exec()
        .map_err(|err| err.to_string())?;

    let globals = lua.globals();
    let collect: Function = globals
        .get("collect")
        .map_err(|_| "plugin must export a collect(ctx) function".to_string())?;

    let metadata = globals
        .get::<Option<Function>>("manifest")
        .map_err(|err| err.to_string())?
        .map(|function| {
            function
                .call::<mlua::Value>(())
                .map_err(|err| err.to_string())
        })
        .transpose()?
        .map(|value| {
            lua.from_value::<PluginMetadataInput>(value)
                .map_err(|err| err.to_string())
        })
        .transpose()?
        .unwrap_or(PluginMetadataInput {
            name: None,
            version: None,
            description: None,
        });

    let context = lua
        .to_value(&build_context())
        .map_err(|err| err.to_string())?;
    let output_value = collect
        .call::<mlua::Value>(context)
        .map_err(|err| err.to_string())?;
    let output: PluginOutput = lua
        .from_value(output_value)
        .map_err(|err| err.to_string())?;
    let metrics = validate_metrics(output.metrics)?;

    let fallback_name = humanize_plugin_id(&sanitize_file_stem(file_name)?);
    Ok(ValidationResult {
        name: truncate_string(metadata.name).unwrap_or(fallback_name),
        description: truncate_string(metadata.description),
        version: truncate_string(metadata.version),
        metrics,
    })
}

fn descriptor_from(plugin: &StoredPlugin, runtime: Option<&PluginRunState>) -> PluginDescriptor {
    let runtime = runtime.cloned().unwrap_or_else(|| PluginRunState {
        status: if plugin.enabled {
            "idle".to_string()
        } else {
            "disabled".to_string()
        },
        ..PluginRunState::default()
    });

    PluginDescriptor {
        id: plugin.id.clone(),
        name: plugin.name.clone(),
        file_name: plugin.file_name.clone(),
        enabled: plugin.enabled,
        description: plugin.description.clone(),
        version: plugin.version.clone(),
        status: runtime.status,
        last_error: runtime.last_error,
        last_run_ms: runtime.last_run_ms,
        last_duration_ms: runtime.last_duration_ms,
        metrics: runtime.metrics,
    }
}

impl PluginEngine {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let root_dir = plugin_root(app)?;
        let scripts_dir = root_dir.join("scripts");
        let manifest_path = root_dir.join(MANIFEST_FILENAME);
        let manifest = load_manifest(&manifest_path);

        Ok(Self {
            scripts_dir,
            manifest_path,
            state: Arc::new(RwLock::new(EngineState {
                plugins: manifest.plugins,
                runtime: HashMap::new(),
            })),
        })
    }

    fn list_plugins(&self) -> Vec<PluginDescriptor> {
        let state = read_lock_or_recover(&self.state);
        let mut plugins = state
            .plugins
            .iter()
            .map(|plugin| descriptor_from(plugin, state.runtime.get(&plugin.id)))
            .collect::<Vec<_>>();
        plugins.sort_by(|left, right| left.name.cmp(&right.name));
        plugins
    }

    fn persist(&self, plugins: &[StoredPlugin]) -> Result<(), String> {
        save_manifest(&self.manifest_path, plugins)
    }

    fn install_plugin(
        &self,
        file_name: String,
        source: String,
    ) -> Result<PluginDescriptor, String> {
        if source.trim().is_empty() {
            return Err("plugin source cannot be empty".to_string());
        }
        if source.len() > MAX_SCRIPT_BYTES {
            return Err(format!("plugin source exceeds {MAX_SCRIPT_BYTES} bytes"));
        }

        let id = sanitize_file_stem(&file_name)?;
        let validation = run_plugin_source(&file_name, &source)?;
        let script_path = self.scripts_dir.join(format!("{id}.lua"));

        let mut state = write_lock_or_recover(&self.state);
        let exists = state.plugins.iter().any(|plugin| plugin.id == id);
        if !exists && state.plugins.len() >= MAX_PLUGINS {
            return Err(format!("plugin registry is full (max {MAX_PLUGINS})"));
        }

        fs::write(&script_path, source).map_err(|err| err.to_string())?;

        let plugin = StoredPlugin {
            id: id.clone(),
            name: validation.name,
            file_name,
            script_path: script_path.to_string_lossy().into_owned(),
            enabled: true,
            description: validation.description,
            version: validation.version,
        };

        if let Some(existing) = state.plugins.iter_mut().find(|existing| existing.id == id) {
            *existing = plugin.clone();
        } else {
            state.plugins.push(plugin.clone());
        }

        state.runtime.insert(
            id.clone(),
            PluginRunState {
                status: "ok".to_string(),
                last_error: None,
                last_run_ms: Some(now_unix_ms()),
                last_duration_ms: Some(0),
                metrics: validation.metrics,
            },
        );

        self.persist(&state.plugins)?;
        Ok(descriptor_from(&plugin, state.runtime.get(&id)))
    }

    fn set_enabled(&self, plugin_id: &str, enabled: bool) -> Result<PluginDescriptor, String> {
        let mut state = write_lock_or_recover(&self.state);
        let plugin = state
            .plugins
            .iter_mut()
            .find(|plugin| plugin.id == plugin_id)
            .ok_or_else(|| format!("plugin '{plugin_id}' was not found"))?;
        plugin.enabled = enabled;
        let plugin_id = plugin.id.clone();
        let plugin_snapshot = plugin.clone();

        let runtime = state.runtime.entry(plugin_id.clone()).or_default();
        runtime.status = if enabled {
            "idle".to_string()
        } else {
            "disabled".to_string()
        };
        if !enabled {
            runtime.last_error = None;
        }
        let runtime_snapshot = runtime.clone();

        self.persist(&state.plugins)?;
        Ok(descriptor_from(&plugin_snapshot, Some(&runtime_snapshot)))
    }

    fn remove_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let mut state = write_lock_or_recover(&self.state);
        let index = state
            .plugins
            .iter()
            .position(|plugin| plugin.id == plugin_id)
            .ok_or_else(|| format!("plugin '{plugin_id}' was not found"))?;

        let plugin = state.plugins.remove(index);
        state.runtime.remove(plugin_id);
        let _ = fs::remove_file(plugin.script_path);
        self.persist(&state.plugins)
    }

    fn poll_once(&self) {
        let plugins = {
            let state = read_lock_or_recover(&self.state);
            state.plugins.clone()
        };

        for plugin in plugins {
            if !plugin.enabled {
                let mut state = write_lock_or_recover(&self.state);
                state.runtime.entry(plugin.id.clone()).or_default().status = "disabled".to_string();
                continue;
            }

            let started = Instant::now();
            let result = fs::read_to_string(&plugin.script_path)
                .map_err(|err| err.to_string())
                .and_then(|source| run_plugin_source(&plugin.file_name, &source));
            let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

            let mut state = write_lock_or_recover(&self.state);
            let runtime = state.runtime.entry(plugin.id.clone()).or_default();
            runtime.last_run_ms = Some(now_unix_ms());
            runtime.last_duration_ms = Some(duration_ms);

            match result {
                Ok(output) => {
                    runtime.status = "ok".to_string();
                    runtime.last_error = None;
                    runtime.metrics = output.metrics;
                }
                Err(error) => {
                    runtime.status = "error".to_string();
                    runtime.last_error = Some(error);
                    runtime.metrics.clear();
                }
            }
        }
    }
}

fn engine(app: &AppHandle) -> Result<Arc<PluginEngine>, String> {
    if let Some(engine) = ENGINE.get() {
        return Ok(Arc::clone(engine));
    }

    let engine = Arc::new(PluginEngine::new(app)?);
    match ENGINE.set(Arc::clone(&engine)) {
        Ok(()) => Ok(engine),
        Err(existing) => Ok(existing),
    }
}

pub fn start_engine(app: AppHandle) -> Result<(), String> {
    let engine = engine(&app)?;
    let worker = Arc::clone(&engine);
    std::thread::spawn(move || loop {
        worker.poll_once();
        std::thread::sleep(Duration::from_secs(PLUGIN_LOOP_SECS));
    });
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn list_plugins(app: AppHandle) -> Result<Vec<PluginDescriptor>, String> {
    Ok(engine(&app)?.list_plugins())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn install_plugin(
    app: AppHandle,
    file_name: String,
    source: String,
) -> Result<PluginDescriptor, String> {
    engine(&app)?.install_plugin(file_name, source)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn set_plugin_enabled(
    app: AppHandle,
    plugin_id: String,
    enabled: bool,
) -> Result<PluginDescriptor, String> {
    engine(&app)?.set_enabled(&plugin_id, enabled)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn remove_plugin(app: AppHandle, plugin_id: String) -> Result<(), String> {
    engine(&app)?.remove_plugin(&plugin_id)
}
