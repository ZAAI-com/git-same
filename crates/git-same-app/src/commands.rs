use git_same_core::api::RepoScanService;
use git_same_core::config::workspace::tilde_collapse_path;
use git_same_core::config::{Config, WorkspaceConfig, WorkspaceManager};
use git_same_core::errors::AppError;
use git_same_core::git::ShellGit;
use git_same_core::ipc::{IpcConfig, StatusFileWriter};
use git_same_core::operations::clone::NoProgress as NoCloneProgress;
use git_same_core::operations::sync::NoSyncProgress;
use git_same_core::provider::NoProgress as NoDiscoveryProgress;
use git_same_core::types::FinderStatus;
use git_same_core::workflows::sync_workspace::{
    execute_prepared_sync, prepare_sync_workspace, SyncWorkspaceRequest,
};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const DAEMON_STALE_AFTER_SECS: u64 = 90;
const FINDER_EXTENSION_ID: &str = "com.zaai.git-same.Badges";

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub root: String,
    pub provider: String,
    pub org_count: usize,
    pub last_sync: Option<String>,
    pub default: bool,
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

#[tauri::command]
pub async fn list_workspaces() -> Result<Vec<WorkspaceSummary>, String> {
    let config = Config::load().map_err(error_string)?;
    let default_workspace = config.default_workspace.clone();
    let workspaces = WorkspaceManager::list().map_err(error_string)?;

    Ok(workspaces
        .iter()
        .map(|workspace| workspace_summary(workspace, default_workspace.as_deref()))
        .collect())
}

#[tauri::command]
pub async fn read_status() -> Result<StatusSnapshot, String> {
    read_status_snapshot().map_err(error_string)
}

#[tauri::command]
pub async fn start_sync(workspace_id: String) -> Result<StatusSnapshot, String> {
    let config = Config::load().map_err(error_string)?;
    let mut workspace =
        WorkspaceManager::resolve(Some(&workspace_id), &config).map_err(error_string)?;
    let progress = NoDiscoveryProgress;

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
        Arc::new(NoCloneProgress),
        Arc::new(NoSyncProgress),
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
    let default = default_workspace
        .map(|value| value == collapsed || same_path_string(value, &workspace.root_path))
        .unwrap_or(false);

    WorkspaceSummary {
        id: collapsed,
        root,
        provider: workspace.provider.kind.display_name().to_string(),
        org_count: workspace.orgs.len(),
        last_sync: workspace.last_synced.clone(),
        default,
    }
}

fn same_path_string(value: &str, path: &Path) -> bool {
    let expanded = shellexpand::tilde(value);
    Path::new(expanded.as_ref()) == path
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
