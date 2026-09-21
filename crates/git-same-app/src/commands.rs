use git_same_core::auth::get_auth_for_provider;
use git_same_core::cache::{CacheManager, DiscoveryCache};
use git_same_core::checks::CheckResult;
use git_same_core::config::workspace::tilde_collapse_path;
use git_same_core::config::{
    Config, ConfigCloneOptions, FilterOptions, SyncMode, WorkspaceConfig, WorkspaceManager,
    WorkspaceProvider,
};
use git_same_core::discovery::DiscoveryOrchestrator;
use git_same_core::domain::RepoPathTemplate;
use git_same_core::errors::{AppError, MonitorAgentError};
use git_same_core::ipc::{remove_symlink_if_present, IpcConfig, StatusFileWriter};
use git_same_core::macos::folder_icon;
use git_same_core::macos::full_disk_access::{self, FullDiskAccess};
use git_same_core::macos::monitor_agent::{self, MonitorAgentState, MonitorAgentStatus};
use git_same_core::progress::{ProgressEvent, ProgressReporter};
use git_same_core::provider::{create_provider, NoProgress};
use git_same_core::setup::{authenticate_provider, discover_org_entries};
use git_same_core::types::{FinderStatus, OwnedRepo, ProviderKind};
use git_same_core::workflows::sync_workspace::{
    execute_prepared_sync, prepare_sync_workspace, SyncWorkspaceRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::Emitter;

const DAEMON_STALE_AFTER_SECS: u64 = 90;
// Referenced only inside the macOS-gated extension_status branch, but kept
// available on every platform for the colocated parser tests.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const FINDER_EXTENSION_ID: &str = "com.zaai.git-same.badges";

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

/// Resolved host-facing IPC config, shared across Tauri command handlers via
/// `tauri::State`. Resolved once in `main.rs` `setup()` so handlers read live
/// status from `~/.config/git-same/finder/` (where the monitor mirrors a real
/// `status.json`) instead of reaching into the app-group container, which would
/// trigger the "access data from other apps" TCC prompt on the non-sandboxed
/// host.
pub struct HostIpc(pub IpcConfig);

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub root: String,
    pub provider: String,
    pub org_count: usize,
    pub last_sync: Option<String>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneOptionsDto {
    pub depth: u32,
    pub branch: String,
    pub recurse_submodules: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterOptionsDto {
    pub include_archived: bool,
    pub include_forks: bool,
    pub orgs: Vec<String>,
    pub exclude_repos: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderConfigDto {
    pub scan_roots: Vec<String>,
    pub max_depth: usize,
    pub exclude_dirs: Vec<String>,
    pub show_ambient: bool,
}

/// Monitor settings as read. `autostart` is informational: it changes only
/// through the dedicated Start and Stop controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorConfigDto {
    pub fullscan_interval_secs: u64,
    pub autostart: bool,
}

/// Monitor settings as saved. Deliberately has no `autostart`, so a stale
/// settings form cannot undo `gisa monitor --stop`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorConfigInput {
    pub fullscan_interval_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub config_path: String,
    pub exists: bool,
    pub structure: String,
    pub concurrency: usize,
    pub sync_mode: String,
    pub default_workspace: Option<String>,
    pub refresh_interval: u64,
    pub clone: CloneOptionsDto,
    pub filters: FilterOptionsDto,
    pub workspaces: Vec<String>,
    pub finder: FinderConfigDto,
    pub monitor: MonitorConfigDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfigInput {
    pub structure: String,
    pub concurrency: usize,
    pub sync_mode: String,
    pub default_workspace: Option<String>,
    pub refresh_interval: u64,
    pub clone: CloneOptionsDto,
    pub filters: FilterOptionsDto,
    pub workspaces: Vec<String>,
    pub finder: FinderConfigDto,
    pub monitor: MonitorConfigInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceProviderDto {
    pub kind: String,
    pub label: String,
    pub api_url: Option<String>,
    pub prefer_ssh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDetailDto {
    pub id: String,
    pub name: String,
    pub root: String,
    pub config_path: String,
    pub provider: WorkspaceProviderDto,
    pub username: String,
    pub orgs: Vec<String>,
    pub include_repos: Vec<String>,
    pub exclude_repos: Vec<String>,
    pub structure: Option<String>,
    pub sync_mode: Option<String>,
    pub clone_options: Option<CloneOptionsDto>,
    pub filters: FilterOptionsDto,
    pub concurrency: Option<usize>,
    pub refresh_interval: Option<u64>,
    pub last_synced: Option<String>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInput {
    pub id: Option<String>,
    pub root: String,
    pub provider: WorkspaceProviderDto,
    pub username: String,
    pub orgs: Vec<String>,
    pub include_repos: Vec<String>,
    pub exclude_repos: Vec<String>,
    pub structure: Option<String>,
    pub sync_mode: Option<String>,
    pub clone_options: Option<CloneOptionsDto>,
    pub filters: FilterOptionsDto,
    pub concurrency: Option<usize>,
    pub refresh_interval: Option<u64>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementCheckDto {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub suggestion: Option<String>,
    pub critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderDiscoveryDto {
    pub username: Option<String>,
    pub orgs: Vec<ProviderOrgDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderOrgDto {
    pub name: String,
    pub repo_count: usize,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceStructureDto {
    pub workspace_id: String,
    pub name: String,
    pub root: String,
    pub provider: String,
    pub host: String,
    pub source: String,
    pub cache_age_secs: Option<u64>,
    pub error: Option<String>,
    pub repos: Vec<WorkspaceStructureRepoDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceStructureRepoDto {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub url: String,
    pub local_path: String,
    pub local_exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    pub status_path: String,
    pub updated_at: Option<String>,
    pub stale: bool,
    pub status: Option<FinderStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionStatus {
    pub installed: bool,
    pub enabled: bool,
}

/// Service status of the managed monitor. The lifecycle lives in
/// `git_same_core::macos::monitor_agent`; this crate only adapts it.
pub type MonitorLaunchAgentStatusDto = MonitorAgentStatus;

/// Full Disk Access as seen by the host and by the monitor. TCC keys the
/// grant on the executable, so both answers are reported and `granted` is
/// the gate the badge setup flow uses (see `fda_gate_passes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FullDiskAccessDto {
    /// This app process's own probe: `granted`, `denied`, `unknown`, or
    /// `not_applicable`.
    pub host: String,
    /// The monitor's stamped answer from `status.json`, when it wrote one.
    pub monitor: Option<bool>,
    /// Whether that status is fresh; a stale monitor may predate a grant.
    pub monitor_fresh: bool,
    /// Whether Finder badges may be enabled.
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressPayload {
    pub workspace_id: String,
    pub event: ProgressEvent,
}

#[tauri::command]
pub async fn list_workspaces() -> Result<Vec<WorkspaceSummary>, String> {
    workspace_summaries().map_err(error_string)
}

#[tauri::command]
pub fn read_app_config() -> Result<AppConfigDto, String> {
    let path = Config::default_path().map_err(error_string)?;
    let exists = path.exists();
    let config = Config::load_from(&path).map_err(error_string)?;
    Ok(app_config_dto(&config, &path, exists))
}

#[tauri::command]
pub fn ensure_config() -> Result<AppConfigDto, String> {
    let path = ensure_config_file().map_err(error_string)?;
    let config = Config::load_from(&path).map_err(error_string)?;
    Ok(app_config_dto(&config, &path, true))
}

/// Saves the settings form as a targeted merge under the monitor control
/// lock. Keys the form does not model (`monitor.autostart`, `[ui]`, comments,
/// unknown keys) keep their persisted values, and a malformed file is never
/// replaced.
#[tauri::command]
pub async fn save_app_config(input: AppConfigInput) -> Result<AppConfigDto, String> {
    let dto = tauri::async_runtime::spawn_blocking(move || save_app_config_inner(input))
        .await
        .map_err(error_string)?
        .map_err(error_string)?;
    nudge_monitor_refresh();
    Ok(dto)
}

fn save_app_config_inner(input: AppConfigInput) -> Result<AppConfigDto, AppError> {
    let path = ensure_config_file()?;
    let settings = app_config_input(input)?;
    with_config_lock(|| git_same_core::config::edit::merge_settings(&path, &settings))?;
    let saved = Config::load_from(&path)?;
    Ok(app_config_dto(&saved, &path, true))
}

/// Serialises every write to the global `config.toml` against the same lock
/// the settings save takes. Two read-modify-write cycles that overlap (Save
/// Settings while the Workspace screen sets a default) would otherwise leave
/// one of the two changes on the floor.
fn with_config_lock<T>(write: impl FnOnce() -> Result<T, AppError>) -> Result<T, AppError> {
    match monitor_agent::with_preference_lock(write) {
        Ok(result) => result,
        Err(e) => Err(AppError::from(e)),
    }
}

#[tauri::command]
pub fn read_workspace(workspace_id: String) -> Result<WorkspaceDetailDto, String> {
    let config = Config::load().map_err(error_string)?;
    let workspace =
        WorkspaceManager::resolve(Some(&workspace_id), &config).map_err(error_string)?;
    Ok(workspace_detail(&workspace, &config))
}

#[tauri::command]
pub fn save_workspace(input: WorkspaceInput) -> Result<WorkspaceDetailDto, String> {
    ensure_config_file().map_err(error_string)?;
    let config = Config::load().map_err(error_string)?;
    let previous = input
        .id
        .as_deref()
        .and_then(|id| WorkspaceManager::resolve(Some(id), &config).ok());
    let was_default = previous
        .as_ref()
        .map(|workspace| workspace_is_default(workspace, config.default_workspace.as_deref()))
        .unwrap_or(false);
    let root = prepare_workspace_root(&input.root).map_err(error_string)?;
    let mut workspace = WorkspaceConfig::new_from_root(&root);

    workspace.provider = provider_input(&input.provider).map_err(error_string)?;
    workspace.username = input.username;
    workspace.orgs = clean_string_list(input.orgs);
    workspace.include_repos = clean_string_list(input.include_repos);
    workspace.exclude_repos = clean_string_list(input.exclude_repos);
    workspace.structure = clean_optional(input.structure);
    workspace.sync_mode = match clean_optional(input.sync_mode) {
        Some(sync_mode) => Some(SyncMode::from_str(&sync_mode).map_err(error_string)?),
        None => None,
    };
    workspace.clone_options = input.clone_options.map(clone_options_input);
    workspace.filters = filter_options_input(input.filters);
    workspace.concurrency = input.concurrency;
    workspace.refresh_interval = input.refresh_interval;
    workspace.last_synced = previous
        .as_ref()
        .and_then(|workspace| workspace.last_synced.clone());

    // `save`/`delete` update the global registry, so both take the lock.
    with_config_lock(|| WorkspaceManager::save(&workspace)).map_err(error_string)?;

    if let Some(previous) = previous {
        if !same_path(&previous.root_path, &workspace.root_path) {
            folder_icon::clear_or_log(&previous.root_path);
            with_config_lock(|| WorkspaceManager::delete(&previous.root_path))
                .map_err(error_string)?;
        }
    }

    if config.ui.custom_folder_icon {
        folder_icon::set_or_log(&workspace.root_path, folder_icon::WORKSPACE_FOLDER_ICNS);
    }

    let collapsed = tilde_collapse_path(&workspace.root_path);
    if input.default {
        with_config_lock(|| Config::save_default_workspace(Some(&collapsed)))
            .map_err(error_string)?;
    } else if was_default {
        with_config_lock(|| Config::save_default_workspace(None)).map_err(error_string)?;
    }

    let config = Config::load().map_err(error_string)?;
    nudge_monitor_refresh();
    Ok(workspace_detail(&workspace, &config))
}

#[tauri::command]
pub fn delete_workspace(workspace_id: String) -> Result<Vec<WorkspaceSummary>, String> {
    let config = Config::load().map_err(error_string)?;
    let workspace =
        WorkspaceManager::resolve(Some(&workspace_id), &config).map_err(error_string)?;
    let was_default = workspace_is_default(&workspace, config.default_workspace.as_deref());

    folder_icon::clear_or_log(&workspace.root_path);
    with_config_lock(|| WorkspaceManager::delete(&workspace.root_path)).map_err(error_string)?;
    if was_default {
        with_config_lock(|| Config::save_default_workspace(None)).map_err(error_string)?;
    }

    nudge_monitor_refresh();
    workspace_summaries().map_err(error_string)
}

#[tauri::command]
pub fn set_default_workspace(
    workspace_id: Option<String>,
) -> Result<Vec<WorkspaceSummary>, String> {
    match workspace_id
        .as_deref()
        .and_then(|id| clean_optional(Some(id.to_string())))
    {
        Some(id) => {
            let config = Config::load().map_err(error_string)?;
            let workspace = WorkspaceManager::resolve(Some(&id), &config).map_err(error_string)?;
            let collapsed = tilde_collapse_path(&workspace.root_path);
            with_config_lock(|| Config::save_default_workspace(Some(&collapsed)))
                .map_err(error_string)?;
        }
        None => with_config_lock(|| Config::save_default_workspace(None)).map_err(error_string)?,
    }

    workspace_summaries().map_err(error_string)
}

#[tauri::command]
pub async fn check_requirements(
    ipc: tauri::State<'_, HostIpc>,
) -> Result<Vec<RequirementCheckDto>, String> {
    let mut checks: Vec<RequirementCheckDto> = git_same_core::checks::check_requirements()
        .await
        .into_iter()
        .map(requirement_check_dto)
        .collect();
    checks.extend(app_requirement_checks(&ipc.0));
    Ok(checks)
}

/// How long a cached status is served before the service is inspected again.
///
/// The cache is refreshed by lifecycle operations and by the file watcher, so
/// it is normally current. The TTL is the backstop for the case where the
/// watcher never fires (a watch that failed to register, an event the
/// platform did not deliver): without it a dead status is served for the rest
/// of the session.
const MONITOR_STATUS_TTL: std::time::Duration = std::time::Duration::from_secs(10);

/// Last known service status. Returned by `monitor_status`, so a fetch made
/// after subscribing can never disagree with an event emitted earlier.
#[derive(Default)]
pub struct MonitorStatusCache {
    cached: std::sync::Mutex<Option<(MonitorAgentStatus, std::time::Instant)>>,
    operations: tokio::sync::Mutex<()>,
}

/// Event carrying a [`MonitorAgentStatus`] after every lifecycle operation
/// and whenever the monitor's runtime files change.
pub const MONITOR_AGENT_UPDATED: &str = "monitor-agent-updated";

/// Caches `status` and tells the frontend.
pub(crate) fn publish_monitor_status(app: &tauri::AppHandle, status: &MonitorAgentStatus) {
    use tauri::Manager;
    if let Some(cache) = app.try_state::<MonitorStatusCache>() {
        *cache.cached.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((status.clone(), std::time::Instant::now()));
    }
    let _ = app.emit(MONITOR_AGENT_UPDATED, status);
}

/// Runs a controller operation off the UI thread and publishes the result.
/// launchctl calls are bounded by timeouts but still block for a while.
async fn run_monitor_operation(
    app: tauri::AppHandle,
    operation: fn() -> Result<MonitorAgentStatus, AppError>,
) -> Result<MonitorAgentStatus, String> {
    use tauri::Manager;
    let state = app.state::<MonitorStatusCache>();
    let _operation = state.operations.lock().await;
    let status = tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(error_string)?
        .map_err(error_string)?;
    publish_monitor_status(&app, &status);
    Ok(status)
}

/// Inspects the service on a worker and publishes the result. Used by the
/// file watcher and on window focus.
pub(crate) fn refresh_monitor_status(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = run_monitor_operation(app, monitor_launch_agent_status_inner).await;
    });
}

/// Restart an already-installed monitor that startup found stale, in the
/// background so the window stays responsive.
///
/// Goes through `run_monitor_operation` like every other lifecycle command, so
/// it takes the same lock as `ensure_monitor_on_startup` instead of racing it,
/// and the result reaches the UI as a `monitor-agent-updated` event.
pub(crate) fn recover_monitor_on_startup(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_monitor_operation(app, restart_monitor_if_installed).await {
            eprintln!("failed to restart monitor after upgrade: {error}");
        }
    });
}

/// One automatic recovery at app startup. Does nothing when suppressed
/// (`GIT_SAME_DISABLE_MONITOR_AUTOSTART=1`, dev launches) or when this is
/// not the real user's default environment, and never enables a service
/// the user stopped.
pub(crate) fn ensure_monitor_on_startup(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = run_monitor_operation(app, || match monitor_agent::auto_ensure(false) {
            Some(Ok(status)) => Ok(status),
            Some(Err(error)) => Ok(monitor_launch_agent_status_inner()?.failed(error.to_string())),
            None => monitor_launch_agent_status_inner(),
        })
        .await;
    });
}

#[tauri::command]
pub async fn monitor_status(
    app: tauri::AppHandle,
    cache: tauri::State<'_, MonitorStatusCache>,
) -> Result<MonitorLaunchAgentStatusDto, String> {
    let cached = cache
        .cached
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    match cached {
        Some((status, at)) if at.elapsed() < MONITOR_STATUS_TTL => Ok(status),
        _ => run_monitor_operation(app, monitor_launch_agent_status_inner).await,
    }
}

#[tauri::command]
pub async fn start_monitor(app: tauri::AppHandle) -> Result<MonitorLaunchAgentStatusDto, String> {
    run_monitor_operation(app, || {
        Ok(monitor_agent::controller_for_current_user(false)?.start()?)
    })
    .await
}

#[tauri::command]
pub async fn stop_monitor(app: tauri::AppHandle) -> Result<MonitorLaunchAgentStatusDto, String> {
    run_monitor_operation(app, || {
        Ok(monitor_agent::controller_for_current_user(false)?.stop()?)
    })
    .await
}

#[tauri::command]
pub async fn restart_monitor(app: tauri::AppHandle) -> Result<MonitorLaunchAgentStatusDto, String> {
    run_monitor_operation(app, || {
        Ok(monitor_agent::controller_for_current_user(false)?.restart()?)
    })
    .await
}

/// Compatibility name for `monitor_status` (always inspects).
#[tauri::command]
pub async fn monitor_launch_agent_status(
    app: tauri::AppHandle,
) -> Result<MonitorLaunchAgentStatusDto, String> {
    run_monitor_operation(app, monitor_launch_agent_status_inner).await
}

/// Compatibility name: installing is an explicit Start.
#[tauri::command]
pub async fn install_monitor_launch_agent(
    app: tauri::AppHandle,
) -> Result<MonitorLaunchAgentStatusDto, String> {
    start_monitor(app).await
}

/// Compatibility name for `restart_monitor`.
#[tauri::command]
pub async fn restart_monitor_launch_agent(
    app: tauri::AppHandle,
) -> Result<MonitorLaunchAgentStatusDto, String> {
    restart_monitor(app).await
}

/// Asks a running monitor to reload the configuration and rescan. A healthy
/// monitor is never restarted, so this is how registry changes reach it.
fn nudge_monitor_refresh() {
    // A redirected configuration (tests) is not the running monitor's.
    if std::env::var_os("GIT_SAME_CONFIG_DIR").is_some() {
        return;
    }
    spawn_refresh_all();
}

/// The monitor is reachable over a Unix socket only.
#[cfg(unix)]
fn spawn_refresh_all() {
    tauri::async_runtime::spawn(async {
        let Ok(ipc) = IpcConfig::default_path() else {
            return;
        };
        let _ = git_same_core::ipc::UnixSocketClient::new(ipc.socket_path())
            .refresh_all()
            .await;
    });
}

/// No socket on this platform: nothing to nudge.
#[cfg(not(unix))]
fn spawn_refresh_all() {}

#[tauri::command]
pub async fn discover_provider_orgs(
    provider: WorkspaceProviderDto,
) -> Result<ProviderDiscoveryDto, String> {
    let provider = provider_input(&provider).map_err(error_string)?;
    if provider.kind != ProviderKind::GitHub {
        return Err("Only GitHub workspace discovery is currently enabled".to_string());
    }

    let auth = authenticate_provider(provider.clone()).await?;
    let orgs = discover_org_entries(provider, auth.token)
        .await?
        .into_iter()
        .map(|org| ProviderOrgDto {
            name: org.name,
            repo_count: org.repo_count,
            selected: org.selected,
        })
        .collect();

    Ok(ProviderDiscoveryDto {
        username: auth.username,
        orgs,
    })
}

#[tauri::command]
pub async fn read_workspace_structure(
    workspace_id: String,
) -> Result<WorkspaceStructureDto, String> {
    read_workspace_structure_inner(workspace_id)
        .await
        .map_err(error_string)
}

#[tauri::command]
pub async fn read_status(ipc: tauri::State<'_, HostIpc>) -> Result<StatusSnapshot, String> {
    read_status_snapshot_with(&ipc.0).map_err(error_string)
}

#[tauri::command]
pub async fn start_sync(
    app: tauri::AppHandle,
    workspace_id: String,
    ipc: tauri::State<'_, HostIpc>,
) -> Result<StatusSnapshot, String> {
    let config = Config::load().map_err(error_string)?;
    let mut workspace =
        WorkspaceManager::resolve(Some(&workspace_id), &config).map_err(error_string)?;
    let progress = sync_progress_reporter(app, workspace_id.clone());

    let prepared = prepare_sync_workspace(
        SyncWorkspaceRequest {
            config: &config,
            workspace: &workspace,
            refresh: false,
            skip_uncommitted: true,
            pull: false,
            concurrency_override: None,
            create_base_path: false,
        },
        &progress,
    )
    .await
    .map_err(error_string)?;

    let outcome = execute_prepared_sync(
        &prepared,
        false,
        Arc::new(progress.clone()),
        Arc::new(progress.clone()),
    )
    .await;
    if outcome
        .clone_summary
        .as_ref()
        .is_some_and(|summary| summary.failed > 0)
        || outcome
            .sync_summary
            .as_ref()
            .is_some_and(|summary| summary.failed > 0)
    {
        return Err("Sync completed with failures".to_string());
    }

    workspace.last_synced = Some(chrono::Utc::now().to_rfc3339());
    WorkspaceManager::save(&workspace).map_err(error_string)?;
    read_status_snapshot_with(&ipc.0).map_err(error_string)
}

fn sync_progress_reporter(app: tauri::AppHandle, workspace_id: String) -> ProgressReporter {
    ProgressReporter::new(move |event| {
        let _ = app.emit(
            "sync-progress",
            SyncProgressPayload {
                workspace_id: workspace_id.clone(),
                event,
            },
        );
    })
}

#[tauri::command]
pub fn extension_status() -> Result<ExtensionStatus, String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/pluginkit")
            .args(["-m", "-v", "-i", FINDER_EXTENSION_ID])
            .output()
            .map_err(|err| format!("pluginkit invocation failed: {err}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_pluginkit_output(&stdout, FINDER_EXTENSION_ID))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(ExtensionStatus {
            installed: false,
            enabled: false,
        })
    }
}

/// Schemes the frontend is allowed to hand to `open`.
///
/// The UI only ever sends the two System Settings panes; `https:` is here so
/// a documentation link does not need a second command. Anything else,
/// including a bare path, a `file:` URL, or a string starting with `-` that
/// `open` would read as a flag, is refused.
const OPENABLE_SCHEMES: [&str; 2] = ["https://", "x-apple.systempreferences:"];

fn is_openable(url: &str) -> bool {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    OPENABLE_SCHEMES
        .iter()
        .any(|scheme| lower.starts_with(scheme) && url.len() > scheme.len())
}

/// Enable the Finder badge extension, refusing until Full Disk Access is
/// granted: without it the monitor cannot read protected folders and the
/// badges would silently stay blank. The gate lives here, not only in the UI,
/// so no frontend path can bypass it.
#[tauri::command]
pub fn enable_finder_extension(ipc: tauri::State<'_, HostIpc>) -> Result<ExtensionStatus, String> {
    let fda = full_disk_access_status_inner(&ipc.0);
    if !fda.granted {
        return Err("Grant Full Disk Access to Git-Same before enabling Finder badges".to_string());
    }
    set_extension_election(ExtensionElection::Use).map_err(|error| error.to_string())?;
    extension_status()
}

#[tauri::command]
pub fn disable_finder_extension() -> Result<ExtensionStatus, String> {
    set_extension_election(ExtensionElection::Ignore).map_err(|error| error.to_string())?;
    extension_status()
}

#[tauri::command]
pub fn full_disk_access_status(
    ipc: tauri::State<'_, HostIpc>,
) -> Result<FullDiskAccessDto, String> {
    Ok(full_disk_access_status_inner(&ipc.0))
}

fn full_disk_access_status_inner(ipc: &IpcConfig) -> FullDiskAccessDto {
    let snapshot = read_status_snapshot_with(ipc).ok();
    full_disk_access_dto(
        full_disk_access::probe(),
        snapshot.as_ref(),
        monitor_runs_as_app_identity(),
    )
}

/// Whether the installed agent runs the monitor as this app's bundle
/// executable, the only program a Full Disk Access grant for Git-Same covers.
///
/// A `Cli`-owned agent execs a copied helper under its own path-based TCC
/// identity, so the app's grant never reaches it. An unknown owner is reported
/// as "not the app": the one gate that consults this fails closed.
fn monitor_runs_as_app_identity() -> bool {
    monitor_launch_agent_status_inner()
        .ok()
        .and_then(|status| status.owner_kind)
        .is_some_and(|owner| owner.is_app())
}

fn full_disk_access_dto(
    host: FullDiskAccess,
    snapshot: Option<&StatusSnapshot>,
    monitor_is_app_identity: bool,
) -> FullDiskAccessDto {
    let monitor_fresh = snapshot.is_some_and(|snapshot| !snapshot.stale);
    let monitor = snapshot
        .and_then(|snapshot| snapshot.status.as_ref())
        .and_then(|status| status.full_disk_access);
    FullDiskAccessDto {
        host: host.as_str().to_string(),
        monitor,
        monitor_fresh,
        granted: fda_gate_passes(host, monitor, monitor_fresh, monitor_is_app_identity),
    }
}

/// The badge-setup gate. A fresh monitor's own answer wins because TCC keys the
/// grant on the monitor executable. Only a definite "granted" passes; unknown
/// never does.
///
/// The awkward arm is a fresh monitor that reports *no* answer: a pre-3.2 build
/// that predates the `full_disk_access` field. Falling back to this process's
/// probe is only sound when that monitor shares this app's TCC identity, so the
/// fallback is withheld unless the agent is app-owned. Without that, badges get
/// enabled against a helper-identity monitor that cannot read the workspace and
/// stay silently blank, which is exactly what this gate exists to prevent.
///
/// A stale or absent monitor keeps the plain host-probe fallback, so a first-run
/// setup with nothing installed yet is never blocked.
fn fda_gate_passes(
    host: FullDiskAccess,
    monitor: Option<bool>,
    monitor_fresh: bool,
    monitor_is_app_identity: bool,
) -> bool {
    match (monitor_fresh, monitor) {
        (true, Some(granted)) => granted,
        (true, None) => monitor_is_app_identity && host == FullDiskAccess::Granted,
        _ => host == FullDiskAccess::Granted,
    }
}

fn full_disk_access_message(fda: &FullDiskAccessDto) -> String {
    match (fda.granted, fda.host.as_str(), fda.monitor) {
        (true, _, _) => "granted to Git-Same",
        (false, "granted", Some(false)) => {
            "granted to the app, but the running monitor lacks it (restart the monitor)"
        }
        // The gate withheld the host-probe fallback: a running monitor that
        // reports no answer is a pre-3.2 build, and the app's grant only covers
        // it once the agent runs this app's executable.
        (false, "granted", None) if fda.monitor_fresh => {
            "granted to the app, but the running monitor is an older build under a \
             different identity (restart the monitor to pick up the grant)"
        }
        (false, "not_applicable", _) => "not applicable on this platform",
        (false, "unknown", None) => "could not be determined",
        _ => "not granted (required for Finder badges)",
    }
    .to_string()
}

/// `pluginkit -e <election>`: the user election macOS stores for an app
/// extension. `use` is what the System Settings toggle sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtensionElection {
    Use,
    Ignore,
}

impl ExtensionElection {
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn pluginkit_arg(self) -> &'static str {
        match self {
            Self::Use => "use",
            Self::Ignore => "ignore",
        }
    }
}

fn set_extension_election(election: ExtensionElection) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("/usr/bin/pluginkit")
            .args(["-e", election.pluginkit_arg(), "-i", FINDER_EXTENSION_ID])
            .output()
            .map_err(|error| AppError::config(format!("pluginkit invocation failed: {error}")))?;
        if output.status.success() {
            return Ok(());
        }
        Err(AppError::config(format!(
            "pluginkit -e {} failed: {}",
            election.pluginkit_arg(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = election;
        Err(AppError::config(
            "Finder extensions are only available on macOS",
        ))
    }
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !is_openable(&url) {
        return Err(format!("refusing to open unsupported URL: {url}"));
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/usr/bin/open")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("open failed: {err}"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("open_url is only implemented on macOS".to_string())
    }
}

/// Off macOS there is no managed service: report that instead of failing so
/// the requirement checks and the UI can render a definite state.
fn monitor_launch_agent_status_inner() -> Result<MonitorLaunchAgentStatusDto, AppError> {
    match monitor_agent::controller_for_current_user(false) {
        Ok(controller) => Ok(controller.inspect()?),
        Err(MonitorAgentError::Unsupported) => Ok(MonitorAgentStatus::unsupported()),
        Err(error) => Err(error.into()),
    }
}

/// Best-effort recovery for the upgrade-skew case: restart the monitor
/// *only if a LaunchAgent is already installed*, so an old (pre-upgrade)
/// monitor process is replaced by the on-disk build, which mirrors a real
/// `status.json` into the host dir. Does nothing when nothing is installed
/// (the user never set up the monitor); it never installs one implicitly.
/// Called from app startup via `recover_monitor_on_startup` when
/// `monitor_needs_startup_recovery` finds evidence of an old build, and exposed
/// to the UI as `restart_monitor_if_agent_installed`.
pub(crate) fn restart_monitor_if_installed() -> Result<MonitorAgentStatus, AppError> {
    let controller = match monitor_agent::controller_for_current_user(false) {
        Ok(controller) => controller,
        Err(MonitorAgentError::Unsupported) => return Ok(MonitorAgentStatus::unsupported()),
        Err(error) => return Err(error.into()),
    };
    let status = controller.inspect()?;
    if status.state == MonitorAgentState::NotInstalled {
        return Ok(status);
    }
    Ok(controller.restart()?)
}

/// Whether an already-installed monitor should be restarted at app launch.
///
/// Two independent signals, either sufficient:
///
/// * A leftover **symlink** at the host status path. Only pre-3.2 monitors
///   create one, so seeing it means an old build is still running.
/// * An installed service that is **running and has completed a scan**, while
///   the host mirror is absent or stale. That is the same old build seen from
///   the other side: it scans and writes the container, but never mirrors.
///
/// The symlink alone is not enough. `read_status_snapshot_with` unlinks it on
/// the first read, and the status watcher performs one within moments of
/// launch, so from the *second* launch onwards there is no symlink left to find
/// and the host status would stay absent indefinitely.
///
/// Requiring a completed scan is what keeps this from restarting a healthy
/// monitor that simply has not finished its first pass yet.
pub(crate) fn monitor_needs_startup_recovery(ipc: &IpcConfig) -> bool {
    // Checked before any snapshot read, which would erase the evidence.
    let host_status_is_symlink = ipc
        .status_file_path()
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    if host_status_is_symlink {
        return true;
    }

    let Ok(agent) = monitor_launch_agent_status_inner() else {
        return false;
    };
    if !agent.running || agent.last_scan.is_none() {
        return false;
    }
    read_status_snapshot_with(ipc)
        .map(|snapshot| snapshot.stale)
        .unwrap_or(true)
}

/// Restart the monitor only when a service is already installed.
///
/// Unlike `restart_monitor`, this never installs one: a plain restart falls
/// back to a full install when nothing is present, which would turn a
/// background recovery attempt into a service the user never asked for. The UI
/// uses this for automatic recovery and keeps `restart_monitor` for the button
/// the user presses deliberately.
#[tauri::command]
pub async fn restart_monitor_if_agent_installed(
    app: tauri::AppHandle,
) -> Result<MonitorLaunchAgentStatusDto, String> {
    run_monitor_operation(app, restart_monitor_if_installed).await
}

// `pluginkit -m -v -i <id>` prints one line per plugin matching the id, or
// nothing if no match. Each line begins with `+` (enabled) or `-` (disabled),
// followed by the plugin id and bundle path. We treat any line containing
// our id as "installed" and the leading `+` as "enabled".
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_pluginkit_output(stdout: &str, target_id: &str) -> ExtensionStatus {
    for line in stdout.lines() {
        if line.contains(target_id) {
            let enabled = line.trim_start().starts_with('+');
            return ExtensionStatus {
                installed: true,
                enabled,
            };
        }
    }
    ExtensionStatus {
        installed: false,
        enabled: false,
    }
}

fn workspace_summaries() -> Result<Vec<WorkspaceSummary>, AppError> {
    let config = Config::load()?;
    let default_workspace = config.default_workspace.clone();
    let workspaces = WorkspaceManager::list()?;

    Ok(workspaces
        .iter()
        .map(|workspace| workspace_summary(workspace, default_workspace.as_deref()))
        .collect())
}

fn ensure_config_file() -> Result<PathBuf, AppError> {
    let path = Config::default_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::config(format!(
                    "Failed to create config directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }
        fs::write(&path, Config::default_toml()).map_err(|error| {
            AppError::config(format!(
                "Failed to write default config at '{}': {error}",
                path.display()
            ))
        })?;
    }
    Ok(path)
}

fn app_config_dto(config: &Config, path: &Path, exists: bool) -> AppConfigDto {
    AppConfigDto {
        config_path: path.display().to_string(),
        exists,
        structure: config.structure.clone(),
        concurrency: config.concurrency,
        sync_mode: sync_mode_label(config.sync_mode),
        default_workspace: config.default_workspace.clone(),
        refresh_interval: config.refresh_interval,
        clone: clone_options_dto(&config.clone),
        filters: filter_options_dto(&config.filters),
        workspaces: config.workspaces.clone(),
        finder: FinderConfigDto {
            scan_roots: config.finder.scan_roots.clone(),
            max_depth: config.finder.max_depth,
            exclude_dirs: config.finder.exclude_dirs.clone(),
            show_ambient: config.finder.show_ambient,
        },
        monitor: MonitorConfigDto {
            fullscan_interval_secs: config.monitor.fullscan_interval_secs,
            autostart: config.monitor.autostart,
        },
    }
}

fn app_config_input(input: AppConfigInput) -> Result<Config, AppError> {
    let mut config = Config {
        structure: input.structure,
        concurrency: input.concurrency,
        sync_mode: SyncMode::from_str(&input.sync_mode).map_err(AppError::config)?,
        default_workspace: input
            .default_workspace
            .and_then(|value| clean_optional(Some(value))),
        refresh_interval: input.refresh_interval,
        clone: clone_options_input(input.clone),
        filters: filter_options_input(input.filters),
        workspaces: clean_string_list(input.workspaces),
        ..Config::default()
    };
    config.finder.scan_roots = clean_string_list(input.finder.scan_roots);
    config.finder.max_depth = input.finder.max_depth;
    config.finder.exclude_dirs = clean_string_list(input.finder.exclude_dirs);
    config.finder.show_ambient = input.finder.show_ambient;
    config.monitor.fullscan_interval_secs = input.monitor.fullscan_interval_secs;
    config.validate()?;
    Ok(config)
}

fn workspace_detail(workspace: &WorkspaceConfig, config: &Config) -> WorkspaceDetailDto {
    WorkspaceDetailDto {
        id: tilde_collapse_path(&workspace.root_path),
        name: workspace_name(&workspace.root_path),
        root: workspace.root_path.display().to_string(),
        config_path: workspace
            .root_path
            .join(".git-same")
            .join("config.toml")
            .display()
            .to_string(),
        provider: provider_dto(&workspace.provider),
        username: workspace.username.clone(),
        orgs: workspace.orgs.clone(),
        include_repos: workspace.include_repos.clone(),
        exclude_repos: workspace.exclude_repos.clone(),
        structure: workspace.structure.clone(),
        sync_mode: workspace.sync_mode.map(sync_mode_label),
        clone_options: workspace.clone_options.as_ref().map(clone_options_dto),
        filters: filter_options_dto(&workspace.filters),
        concurrency: workspace.concurrency,
        refresh_interval: workspace.refresh_interval,
        last_synced: workspace.last_synced.clone(),
        default: workspace_is_default(workspace, config.default_workspace.as_deref()),
    }
}

fn provider_dto(provider: &WorkspaceProvider) -> WorkspaceProviderDto {
    WorkspaceProviderDto {
        kind: provider.kind.slug().to_string(),
        label: provider.kind.display_name().to_string(),
        api_url: provider.api_url.clone(),
        prefer_ssh: provider.prefer_ssh,
    }
}

fn provider_input(input: &WorkspaceProviderDto) -> Result<WorkspaceProvider, String> {
    let kind = ProviderKind::from_str(&input.kind)?;
    Ok(WorkspaceProvider {
        kind,
        api_url: input
            .api_url
            .clone()
            .and_then(|value| clean_optional(Some(value))),
        prefer_ssh: input.prefer_ssh,
    })
}

fn clone_options_dto(options: &ConfigCloneOptions) -> CloneOptionsDto {
    CloneOptionsDto {
        depth: options.depth,
        branch: options.branch.clone(),
        recurse_submodules: options.recurse_submodules,
    }
}

fn clone_options_input(input: CloneOptionsDto) -> ConfigCloneOptions {
    ConfigCloneOptions {
        depth: input.depth,
        branch: input.branch,
        recurse_submodules: input.recurse_submodules,
    }
}

fn filter_options_dto(filters: &FilterOptions) -> FilterOptionsDto {
    FilterOptionsDto {
        include_archived: filters.include_archived,
        include_forks: filters.include_forks,
        orgs: filters.orgs.clone(),
        exclude_repos: filters.exclude_repos.clone(),
    }
}

fn filter_options_input(input: FilterOptionsDto) -> FilterOptions {
    FilterOptions {
        include_archived: input.include_archived,
        include_forks: input.include_forks,
        orgs: clean_string_list(input.orgs),
        exclude_repos: clean_string_list(input.exclude_repos),
    }
}

fn sync_mode_label(sync_mode: SyncMode) -> String {
    match sync_mode {
        SyncMode::Fetch => "fetch",
        SyncMode::Pull => "pull",
    }
    .to_string()
}

fn app_requirement_checks(ipc: &IpcConfig) -> Vec<RequirementCheckDto> {
    let config_path = match Config::default_path() {
        Ok(path) => path,
        Err(error) => {
            return vec![RequirementCheckDto {
                name: "Config file".to_string(),
                passed: false,
                message: error.to_string(),
                suggestion: Some("Check HOME or GIT_SAME_CONFIG_DIR".to_string()),
                critical: true,
            }]
        }
    };
    let config_exists = config_path.exists();
    let mut checks = vec![RequirementCheckDto {
        name: "Config file".to_string(),
        passed: config_exists,
        message: if config_exists {
            config_path.display().to_string()
        } else {
            "not created".to_string()
        },
        suggestion: (!config_exists).then(|| "Create the default Git-Same config".to_string()),
        critical: true,
    }];

    let snapshot = read_status_snapshot_with(ipc).ok();
    let monitor_agent = monitor_launch_agent_status_inner().ok();
    checks.push(RequirementCheckDto {
        name: "Monitor".to_string(),
        passed: monitor_requirement_passed(
            monitor_agent.as_ref(),
            snapshot.as_ref(),
            env!("CARGO_PKG_VERSION"),
        ),
        message: monitor_requirement_message(
            monitor_agent.as_ref(),
            snapshot.as_ref(),
            env!("CARGO_PKG_VERSION"),
        ),
        suggestion: monitor_requirement_suggestion(
            monitor_agent.as_ref(),
            snapshot.as_ref(),
            env!("CARGO_PKG_VERSION"),
        ),
        critical: false,
    });

    let extension = extension_status().ok();
    checks.push(RequirementCheckDto {
        name: "Finder extension".to_string(),
        passed: extension
            .as_ref()
            .is_some_and(|extension| extension.installed && extension.enabled),
        message: match extension {
            Some(ExtensionStatus {
                installed: true,
                enabled: true,
            }) => "installed and enabled".to_string(),
            Some(ExtensionStatus {
                installed: true,
                enabled: false,
            }) => "installed but disabled".to_string(),
            Some(_) => "not installed".to_string(),
            None => "unable to check".to_string(),
        },
        suggestion: Some("Enable Git-Same Badges in System Settings".to_string()),
        critical: false,
    });

    let fda = full_disk_access_dto(
        full_disk_access::probe(),
        snapshot.as_ref(),
        monitor_agent
            .as_ref()
            .and_then(|status| status.owner_kind)
            .is_some_and(|owner| owner.is_app()),
    );
    checks.push(RequirementCheckDto {
        name: "Full Disk Access".to_string(),
        passed: fda.granted,
        message: full_disk_access_message(&fda),
        suggestion: (!fda.granted).then(|| {
            "Grant Full Disk Access to Git-Same in System Settings, then quit and reopen the app"
                .to_string()
        }),
        critical: false,
    });

    checks
}

/// Starting counts as healthy: a long first scan is not a problem to fix.
fn monitor_is_healthy(agent: &MonitorLaunchAgentStatusDto) -> bool {
    matches!(
        agent.state,
        MonitorAgentState::Running | MonitorAgentState::Starting
    )
}

/// The monitor's build version when the mirrored status reports one that
/// differs from the app's own build, or `None` when they match or none is
/// known. Older monitors that predate the `monitor_version` field, or that are
/// too old to mirror a readable status at all, report `None` here; the agent
/// state arms cover that case instead.
fn monitor_version_mismatch(
    snapshot: Option<&StatusSnapshot>,
    app_version: &str,
) -> Option<String> {
    snapshot
        .and_then(|snapshot| snapshot.status.as_ref())
        .and_then(|status| status.monitor_version.clone())
        .filter(|version| version != app_version)
}

/// Whether the Monitor requirement is satisfied. Mirrors the conditions that
/// `monitor_requirement_message`/`monitor_requirement_suggestion` treat as
/// problems, including a build-version skew, so the row's pass state never
/// contradicts its own message and suggestion.
fn monitor_requirement_passed(
    agent: Option<&MonitorLaunchAgentStatusDto>,
    snapshot: Option<&StatusSnapshot>,
    app_version: &str,
) -> bool {
    agent.is_some_and(monitor_is_healthy)
        && monitor_version_mismatch(snapshot, app_version).is_none()
}

fn monitor_requirement_message(
    agent: Option<&MonitorLaunchAgentStatusDto>,
    snapshot: Option<&StatusSnapshot>,
    app_version: &str,
) -> String {
    match agent {
        Some(agent) if monitor_is_healthy(agent) => {
            match monitor_version_mismatch(snapshot, app_version) {
                Some(skew) => format!(
                    "Monitor is running a different build ({}) than the app ({})",
                    skew, app_version
                ),
                None => agent
                    .detail
                    .clone()
                    .unwrap_or_else(|| agent.message.clone()),
            }
        }
        Some(agent) => agent
            .detail
            .clone()
            .unwrap_or_else(|| agent.message.clone()),
        None => "Unable to inspect the background monitor".to_string(),
    }
}

fn monitor_requirement_suggestion(
    agent: Option<&MonitorLaunchAgentStatusDto>,
    snapshot: Option<&StatusSnapshot>,
    app_version: &str,
) -> Option<String> {
    let agent = agent?;
    match agent.state {
        MonitorAgentState::Running | MonitorAgentState::Starting => {
            monitor_version_mismatch(snapshot, app_version)
                .map(|_| "Restart the monitor so it runs the same build as the app".to_string())
        }
        MonitorAgentState::Deferred => {
            Some("Nothing to do: it starts at your next login".to_string())
        }
        MonitorAgentState::Disabled => Some("Start monitoring to see Finder badges".to_string()),
        MonitorAgentState::NotInstalled | MonitorAgentState::Stopped => {
            Some("Start the Git-Same monitor".to_string())
        }
        MonitorAgentState::Failed => Some("Start the monitor again to repair it".to_string()),
        MonitorAgentState::Unsupported => Some("Run `gisa monitor` in a terminal".to_string()),
    }
}

async fn read_workspace_structure_inner(
    workspace_id: String,
) -> Result<WorkspaceStructureDto, AppError> {
    let config = Config::load()?;
    let workspace = WorkspaceManager::resolve(Some(&workspace_id), &config)?;
    let base_path = workspace.expanded_base_path();
    let structure = workspace
        .structure
        .clone()
        .unwrap_or_else(|| config.structure.clone());
    let provider_name = workspace.provider.kind.slug().to_string();
    let orchestrator = workspace_orchestrator(&workspace, &config, structure.clone());

    let mut source = "cache".to_string();
    let mut cache_age_secs = None;
    let mut error = None;
    let repos = match load_structure_cache(&workspace, &orchestrator) {
        Ok(Some((repos, age_secs))) => {
            cache_age_secs = Some(age_secs);
            repos
        }
        Ok(None) | Err(_) => match discover_structure_repos(&workspace, &orchestrator).await {
            Ok(repos) => {
                source = "remote".to_string();
                save_structure_cache(&workspace, &provider_name, &repos);
                repos
            }
            Err(err) => {
                source = "unavailable".to_string();
                error = Some(err);
                Vec::new()
            }
        },
    };

    Ok(WorkspaceStructureDto {
        workspace_id: tilde_collapse_path(&workspace.root_path),
        name: workspace_name(&workspace.root_path),
        root: base_path.display().to_string(),
        provider: workspace.provider.kind.display_name().to_string(),
        host: provider_host(&workspace.provider),
        source,
        cache_age_secs,
        error,
        repos: structure_repo_dtos(&repos, &base_path, &provider_name, &structure),
    })
}

fn workspace_orchestrator(
    workspace: &WorkspaceConfig,
    _config: &Config,
    structure: String,
) -> DiscoveryOrchestrator {
    let mut filters = workspace.filters.clone();
    if !workspace.orgs.is_empty() {
        filters.orgs = workspace.orgs.clone();
    }
    filters.exclude_repos = workspace.exclude_repos.clone();
    DiscoveryOrchestrator::new(filters, structure)
}

fn load_structure_cache(
    workspace: &WorkspaceConfig,
    orchestrator: &DiscoveryOrchestrator,
) -> anyhow::Result<Option<(Vec<OwnedRepo>, u64)>> {
    let Some(cache) = CacheManager::for_workspace(&workspace.root_path)?.load()? else {
        return Ok(None);
    };
    let age_secs = cache.age_secs();
    let options = orchestrator.to_discovery_options();
    let repos = cache
        .repos
        .values()
        .flat_map(|provider_repos| provider_repos.iter())
        .filter(|owned| {
            options.should_include_org(&owned.owner) && options.should_include(&owned.repo)
        })
        .cloned()
        .collect();
    Ok(Some((repos, age_secs)))
}

async fn discover_structure_repos(
    workspace: &WorkspaceConfig,
    orchestrator: &DiscoveryOrchestrator,
) -> Result<Vec<OwnedRepo>, String> {
    let provider_cfg = workspace.provider.clone();
    let auth = tokio::task::spawn_blocking(move || get_auth_for_provider(&provider_cfg))
        .await
        .map_err(|err| format!("Auth task failed: {err}"))?
        .map_err(|err| err.to_string())?;
    let provider =
        create_provider(&workspace.provider, &auth.token).map_err(|err| err.to_string())?;
    orchestrator
        .discover(provider.as_ref(), &NoProgress)
        .await
        .map_err(|err| err.to_string())
}

fn save_structure_cache(workspace: &WorkspaceConfig, provider_name: &str, repos: &[OwnedRepo]) {
    let Ok(cache_manager) = CacheManager::for_workspace(&workspace.root_path) else {
        return;
    };
    let mut repos_by_provider = HashMap::new();
    repos_by_provider.insert(provider_name.to_string(), repos.to_vec());
    let cache = DiscoveryCache::new(workspace.username.clone(), repos_by_provider);
    let _ = cache_manager.save(&cache);
}

fn structure_repo_dtos(
    repos: &[OwnedRepo],
    base_path: &Path,
    provider_name: &str,
    structure: &str,
) -> Vec<WorkspaceStructureRepoDto> {
    let template = RepoPathTemplate::new(structure.to_string());
    let mut dtos: Vec<_> = repos
        .iter()
        .map(|owned| {
            let local_path = template.render_owned_repo(base_path, owned, provider_name);
            WorkspaceStructureRepoDto {
                owner: owned.owner.clone(),
                name: owned.repo.name.clone(),
                full_name: owned.repo.full_name.clone(),
                url: repo_url(owned),
                local_exists: local_path.exists(),
                local_path: local_path.display().to_string(),
            }
        })
        .collect();
    dtos.sort_by(|left, right| left.full_name.cmp(&right.full_name));
    dtos
}

fn repo_url(owned: &OwnedRepo) -> String {
    if !owned.repo.clone_url.is_empty() {
        return owned.repo.clone_url.trim_end_matches(".git").to_string();
    }
    format!("https://github.com/{}", owned.repo.full_name)
}

fn provider_host(provider: &WorkspaceProvider) -> String {
    match provider.kind {
        ProviderKind::GitHub => "github.com".to_string(),
        _ => provider
            .api_url
            .clone()
            .unwrap_or_else(|| provider.kind.default_api_url().to_string())
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string(),
    }
}

fn requirement_check_dto(check: CheckResult) -> RequirementCheckDto {
    RequirementCheckDto {
        name: check.name,
        passed: check.passed,
        message: check.message,
        suggestion: check.suggestion,
        critical: check.critical,
    }
}

/// `stale` describes badge-data freshness only. Whether a monitor process
/// is running is a separate question answered by the monitor status.
pub(crate) fn read_status_snapshot_with(ipc: &IpcConfig) -> Result<StatusSnapshot, AppError> {
    ipc.ensure_dir()?;
    let status_path = ipc.status_file_path();
    // Older layouts symlinked status.json into the app-group container;
    // following that link would re-trigger the "access data from other apps"
    // TCC prompt, so unlink it before anything dereferences the path. The
    // monitor's next mirror write recreates a real file here.
    remove_symlink_if_present(&status_path)?;
    // Single parse: None covers both a missing and a corrupt status file.
    let status = StatusFileWriter::new(status_path.clone()).read().ok();
    let modified = fs::metadata(&status_path)
        .ok()
        .and_then(|meta| meta.modified().ok());
    let updated_at = modified.map(system_time_to_rfc3339);
    let stale_by_age = modified
        .map(|modified| {
            modified
                .elapsed()
                .unwrap_or(Duration::from_secs(DAEMON_STALE_AFTER_SECS + 1))
                > Duration::from_secs(DAEMON_STALE_AFTER_SECS)
        })
        .unwrap_or(true);
    // A file we cannot parse carries no usable badge data, so it is stale
    // regardless of its mtime.
    let stale = stale_by_age || status.is_none();

    Ok(StatusSnapshot {
        status_path: status_path.display().to_string(),
        updated_at,
        stale,
        status,
    })
}

fn workspace_summary(
    workspace: &WorkspaceConfig,
    default_workspace: Option<&str>,
) -> WorkspaceSummary {
    let collapsed = tilde_collapse_path(&workspace.root_path);
    let root = workspace.root_path.display().to_string();
    let default = workspace_is_default(workspace, default_workspace);

    WorkspaceSummary {
        id: collapsed,
        name: workspace_name(&workspace.root_path),
        root,
        provider: workspace.provider.kind.display_name().to_string(),
        org_count: workspace.orgs.len(),
        last_sync: workspace.last_synced.clone(),
        default,
    }
}

fn workspace_is_default(workspace: &WorkspaceConfig, default_workspace: Option<&str>) -> bool {
    let collapsed = tilde_collapse_path(&workspace.root_path);
    default_workspace
        .map(|value| value == collapsed || same_path_string(value, &workspace.root_path))
        .unwrap_or(false)
}

fn workspace_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("Workspace"))
        .to_string()
}

fn prepare_workspace_root(value: &str) -> Result<PathBuf, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::config("Workspace root is required"));
    }
    let expanded = shellexpand::tilde(trimmed);
    let path = PathBuf::from(expanded.as_ref());
    fs::create_dir_all(&path).map_err(|error| {
        AppError::config(format!(
            "Failed to create workspace root '{}': {error}",
            path.display()
        ))
    })?;
    Ok(fs::canonicalize(&path).unwrap_or(path))
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn same_path_string(value: &str, path: &Path) -> bool {
    let expanded = shellexpand::tilde(value);
    Path::new(expanded.as_ref()) == path
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn clean_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| clean_optional(Some(value)))
        .collect()
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    datetime.to_rfc3339()
}

fn error_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
