use git_same_core::api::RepoScanService;
use git_same_core::checks::CheckResult;
use git_same_core::config::workspace::tilde_collapse_path;
use git_same_core::config::{
    Config, ConfigCloneOptions, FilterOptions, SyncMode, WorkspaceConfig, WorkspaceManager,
    WorkspaceProvider,
};
use git_same_core::errors::AppError;
use git_same_core::git::ShellGit;
use git_same_core::ipc::{IpcConfig, StatusFileWriter};
use git_same_core::progress::{ProgressEvent, ProgressReporter};
use git_same_core::setup::{authenticate_provider, discover_org_entries};
use git_same_core::types::{FinderStatus, ProviderKind};
use git_same_core::workflows::sync_workspace::{
    execute_prepared_sync, prepare_sync_workspace, SyncWorkspaceRequest,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::Emitter;

const DAEMON_STALE_AFTER_SECS: u64 = 90;
const FINDER_EXTENSION_ID: &str = "com.zaai.git-same.Badges";

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

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

#[tauri::command]
pub fn save_app_config(input: AppConfigInput) -> Result<AppConfigDto, String> {
    let path = ensure_config_file().map_err(error_string)?;
    let config = app_config_input(input).map_err(error_string)?;
    let content = toml::to_string_pretty(&config)
        .map_err(|error| format!("Failed to serialize config: {error}"))?;
    fs::write(&path, content)
        .map_err(|error| format!("Failed to write config at '{}': {error}", path.display()))?;
    Ok(app_config_dto(&config, &path, true))
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

    WorkspaceManager::save(&workspace).map_err(error_string)?;

    if let Some(previous) = previous {
        if !same_path(&previous.root_path, &workspace.root_path) {
            WorkspaceManager::delete(&previous.root_path).map_err(error_string)?;
        }
    }

    let collapsed = tilde_collapse_path(&workspace.root_path);
    if input.default {
        Config::save_default_workspace(Some(&collapsed)).map_err(error_string)?;
    } else if was_default {
        Config::save_default_workspace(None).map_err(error_string)?;
    }

    let config = Config::load().map_err(error_string)?;
    Ok(workspace_detail(&workspace, &config))
}

#[tauri::command]
pub fn delete_workspace(workspace_id: String) -> Result<Vec<WorkspaceSummary>, String> {
    let config = Config::load().map_err(error_string)?;
    let workspace =
        WorkspaceManager::resolve(Some(&workspace_id), &config).map_err(error_string)?;
    let was_default = workspace_is_default(&workspace, config.default_workspace.as_deref());

    WorkspaceManager::delete(&workspace.root_path).map_err(error_string)?;
    if was_default {
        Config::save_default_workspace(None).map_err(error_string)?;
    }

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
            Config::save_default_workspace(Some(&collapsed)).map_err(error_string)?;
        }
        None => Config::save_default_workspace(None).map_err(error_string)?,
    }

    workspace_summaries().map_err(error_string)
}

#[tauri::command]
pub async fn check_requirements() -> Result<Vec<RequirementCheckDto>, String> {
    let mut checks: Vec<RequirementCheckDto> = git_same_core::checks::check_requirements()
        .await
        .into_iter()
        .map(requirement_check_dto)
        .collect();
    checks.extend(app_requirement_checks());
    Ok(checks)
}

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
pub async fn read_status() -> Result<StatusSnapshot, String> {
    read_status_snapshot().map_err(error_string)
}

#[tauri::command]
pub async fn start_sync(
    app: tauri::AppHandle,
    workspace_id: String,
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
    let ipc = IpcConfig::default_path().map_err(error_string)?;
    read_status_snapshot_with(&config, &ipc).map_err(error_string)
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

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
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
        let _ = url;
        Err("open_url is only implemented on macOS".to_string())
    }
}

// `pluginkit -m -v -i <id>` prints one line per plugin matching the id, or
// nothing if no match. Each line begins with `+` (enabled) or `-` (disabled),
// followed by the plugin id and bundle path. We treat any line containing
// our id as "installed" and the leading `+` as "enabled".
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

fn app_requirement_checks() -> Vec<RequirementCheckDto> {
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

    let snapshot = read_status_snapshot().ok();
    checks.push(RequirementCheckDto {
        name: "Monitor".to_string(),
        passed: snapshot.as_ref().is_some_and(|snapshot| !snapshot.stale),
        message: snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.updated_at.clone())
            .unwrap_or_else(|| "no fresh status file".to_string()),
        suggestion: snapshot
            .as_ref()
            .map(|snapshot| snapshot.stale)
            .unwrap_or(true)
            .then(|| "Load the Git-Same LaunchAgent or run gisa monitor --foreground".to_string()),
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

    let fda_needed = snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.status.as_ref())
        .is_some_and(|status| !status.workspaces.is_empty() && status.repos.is_empty());
    checks.push(RequirementCheckDto {
        name: "Full Disk Access".to_string(),
        passed: !fda_needed,
        message: if fda_needed {
            "no repositories visible to the monitor".to_string()
        } else {
            "not currently required".to_string()
        },
        suggestion: fda_needed
            .then(|| "Grant Full Disk Access to Git-Same in System Settings".to_string()),
        critical: false,
    });

    checks
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

pub(crate) fn read_status_snapshot() -> Result<StatusSnapshot, AppError> {
    let config = Config::load()?;
    let ipc = IpcConfig::default_path()?;
    read_status_snapshot_with(&config, &ipc)
}

fn read_status_snapshot_with(config: &Config, ipc: &IpcConfig) -> Result<StatusSnapshot, AppError> {
    ipc.ensure_dir()?;
    let status_path = ipc.status_file_path();
    let writer = StatusFileWriter::new(status_path.clone());
    let metadata = fs::metadata(&status_path).ok();
    let updated_at = metadata
        .as_ref()
        .and_then(|meta| meta.modified().ok())
        .map(system_time_to_rfc3339);
    let stale_by_age = metadata
        .as_ref()
        .and_then(|meta| meta.modified().ok())
        .map(|modified| {
            modified
                .elapsed()
                .unwrap_or(Duration::from_secs(DAEMON_STALE_AFTER_SECS + 1))
                > Duration::from_secs(DAEMON_STALE_AFTER_SECS)
        })
        .unwrap_or(true);
    let monitor_alive = if writer.exists() {
        writer
            .read()
            .map(|status| is_process_alive(status.daemon_pid))
            .unwrap_or(false)
    } else {
        false
    };
    let stale = stale_by_age || !monitor_alive;
    let status = if writer.exists() && !stale {
        Some(writer.read()?)
    } else if config.workspaces.is_empty() {
        writer.exists().then(|| writer.read()).transpose()?
    } else {
        Some(scan_foreground_status(config))
    };

    Ok(StatusSnapshot {
        status_path: status_path.display().to_string(),
        updated_at,
        stale,
        status,
    })
}

fn scan_foreground_status(config: &Config) -> FinderStatus {
    let git = ShellGit::new();
    RepoScanService::new(&git, config)
        .scan_all(std::process::id())
        .unwrap_or_else(|_| FinderStatus::new(std::process::id(), chrono::Utc::now().to_rfc3339()))
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

fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

fn error_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
