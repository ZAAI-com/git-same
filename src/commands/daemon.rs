//! Daemon command handler.
//!
//! Runs a background daemon that monitors workspace repositories,
//! computes Finder badge status, and writes the status JSON file.
//! Listens on a Unix socket for refresh requests from the Finder extension.

use crate::cli::DaemonArgs;
use crate::config::{Config, WorkspaceStore};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::Result;
use crate::git::{GitOperations, ShellGit};
use crate::ipc::{IpcConfig, StatusFileWriter};
use crate::output::Output;
use crate::types::finder_status::{
    compute_badge, matches_important_pattern, FinderBranchInfo, FinderRepoStatus, FinderStatus,
    FinderWorkspaceInfo, FinderWorktreeInfo, OrgFolderInfo, DEFAULT_IMPORTANT_IGNORED_PATTERNS,
};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Run the daemon command.
pub async fn run(args: &DaemonArgs, config: &Config, output: &Output) -> Result<()> {
    let ipc_config = IpcConfig::default_path()?;
    ipc_config.ensure_dir()?;

    // Handle --status: check if daemon is running
    if args.status {
        return show_status(&ipc_config, output);
    }

    // Handle --stop: terminate running daemon
    if args.stop {
        return stop_daemon(&ipc_config, output);
    }

    // Start the daemon
    info!("Starting git-same daemon");
    output.info("Starting git-same daemon...");

    let status_writer = StatusFileWriter::new(ipc_config.status_file_path());
    let git = ShellGit::new();
    let pid = std::process::id();

    // Initial scan
    let finder_status = scan_all_workspaces(config, &git, pid)?;
    status_writer.write(&finder_status)?;
    info!(
        repos = finder_status.repos.len(),
        "Initial scan complete, status written"
    );
    output.info(&format!(
        "Monitoring {} repos. Status: {}",
        finder_status.repos.len(),
        ipc_config.status_file_path().display()
    ));

    // Set up Unix socket listener
    #[cfg(unix)]
    let socket_listener = crate::ipc::UnixSocketListener::new(ipc_config.socket_path());

    #[cfg(unix)]
    let tokio_listener = socket_listener.bind().await?;

    // Main daemon loop
    let interval = tokio::time::Duration::from_secs(args.interval);

    loop {
        tokio::select! {
            // Wait for the polling interval
            _ = tokio::time::sleep(interval) => {
                debug!("Polling interval reached, scanning...");
                match scan_all_workspaces(config, &git, pid) {
                    Ok(status) => {
                        if let Err(e) = status_writer.write(&status) {
                            error!(error = %e, "Failed to write status file");
                        } else {
                            debug!(repos = status.repos.len(), "Scan complete");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Scan failed");
                    }
                }
            },
            // Accept socket connections for refresh requests
            result = tokio_listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let config_clone = config.clone();
                        let writer_path = status_writer.path().to_path_buf();
                        tokio::spawn(async move {
                            handle_socket_connection(stream, &config_clone, pid, &writer_path).await;
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to accept socket connection");
                    }
                }
            },
            // Handle shutdown signal
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                output.info("Daemon shutting down...");
                socket_listener.cleanup();
                break;
            },
        }
    }

    Ok(())
}

/// Handle a single socket connection.
#[cfg(unix)]
async fn handle_socket_connection(
    mut stream: tokio::net::UnixStream,
    config: &Config,
    pid: u32,
    status_path: &Path,
) {
    use crate::ipc::unix_socket::DaemonCommand;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    match reader.read_line(&mut line).await {
        Ok(0) => return, // connection closed
        Ok(_) => {}
        Err(e) => {
            debug!(error = %e, "Failed to read from socket");
            return;
        }
    }

    let cmd = DaemonCommand::parse(&line);
    let git = ShellGit::new();

    let response = match cmd {
        DaemonCommand::Ping => "PONG\n".to_string(),
        DaemonCommand::RefreshAll | DaemonCommand::Refresh(_) => {
            if let DaemonCommand::Refresh(ref path) = cmd {
                debug!(path = %path.display(), "Refresh requested");
            }
            match scan_all_workspaces(config, &git, pid) {
                Ok(status) => {
                    let file_writer = StatusFileWriter::new(status_path.to_path_buf());
                    let _ = file_writer.write(&status);
                }
                Err(e) => error!(error = %e, "Refresh failed"),
            }
            "OK\n".to_string()
        }
        DaemonCommand::Status => {
            let file_writer = StatusFileWriter::new(status_path.to_path_buf());
            match file_writer.read() {
                Ok(status) => {
                    serde_json::to_string_pretty(&status).unwrap_or_else(|_| "ERROR\n".to_string())
                }
                Err(_) => "ERROR\n".to_string(),
            }
        }
        DaemonCommand::Unknown(cmd) => {
            format!("UNKNOWN: {}\n", cmd)
        }
    };

    let _ = writer.write_all(response.as_bytes()).await;
    let _ = writer.flush().await;
}

/// Scan all configured workspaces and build the FinderStatus.
fn scan_all_workspaces(config: &Config, git: &ShellGit, pid: u32) -> Result<FinderStatus> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut status = FinderStatus::new(pid, timestamp);

    for ws_path in &config.workspaces {
        let expanded = shellexpand::tilde(ws_path).to_string();
        let root = PathBuf::from(&expanded);
        if !root.exists() {
            debug!(path = %root.display(), "Workspace root does not exist, skipping");
            continue;
        }

        // Load workspace config
        let ws_config = match WorkspaceStore::load(&root) {
            Ok(ws) => ws,
            Err(e) => {
                debug!(
                    path = %root.display(),
                    error = %e,
                    "Failed to load workspace config, skipping"
                );
                continue;
            }
        };

        let base_path = ws_config.expanded_base_path();
        // Use directory name as workspace name
        let ws_name = base_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(ws_path)
            .to_string();
        let structure = ws_config.structure.as_deref().unwrap_or(&config.structure);

        // orgs is Vec<String> directly
        let org_names: Vec<String> = ws_config.orgs.clone();

        status.workspaces.push(FinderWorkspaceInfo {
            name: ws_name.clone(),
            root: base_path.clone(),
            orgs: org_names.clone(),
        });

        // Add org folder entries — scan filesystem for org directories
        // If orgs list is specified, use it; otherwise discover from directory listing
        let org_dirs: Vec<String> = if org_names.is_empty() {
            // Discover org directories from filesystem
            std::fs::read_dir(&base_path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                        .filter(|e| {
                            e.file_name()
                                .to_str()
                                .map(|n| !n.starts_with('.'))
                                .unwrap_or(false)
                        })
                        .filter_map(|e| e.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            org_names.clone()
        };

        for org_name in &org_dirs {
            let org_path = base_path.join(org_name);
            if org_path.exists() {
                status.org_folders.push(OrgFolderInfo {
                    path: org_path,
                    org: org_name.clone(),
                    workspace: ws_name.clone(),
                });
            }
        }

        // Scan local repos
        let orchestrator =
            DiscoveryOrchestrator::new(ws_config.filters.clone(), structure.to_string());
        let local_repos = orchestrator.scan_local(&base_path, git);

        for (repo_path, org, _name) in local_repos {
            let repo_status = scan_single_repo(git, &repo_path, Some(&ws_name), Some(&org));
            status.repos.push(repo_status);
        }
    }

    Ok(status)
}

/// Scan a single repository and build its FinderRepoStatus.
fn scan_single_repo(
    git: &dyn GitOperations,
    repo_path: &Path,
    workspace: Option<&str>,
    org: Option<&str>,
) -> FinderRepoStatus {
    // Get basic status
    let repo_status = git
        .status(repo_path)
        .unwrap_or_else(|_| crate::git::RepoStatus {
            branch: "unknown".to_string(),
            is_uncommitted: false,
            ahead: 0,
            behind: 0,
            has_untracked: false,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
        });

    // Get branches
    let branches: Vec<FinderBranchInfo> = git
        .list_branches(repo_path)
        .unwrap_or_default()
        .into_iter()
        .map(|b| FinderBranchInfo {
            name: b.name,
            upstream: b.upstream,
            ahead: b.ahead,
            behind: b.behind,
            synced: b.is_synced,
        })
        .collect();

    let all_branches_synced = branches.iter().all(|b| b.synced);

    // Get remotes
    let remotes: Vec<crate::types::finder_status::FinderRemoteInfo> = git
        .list_remotes(repo_path)
        .unwrap_or_default()
        .into_iter()
        .map(|r| crate::types::finder_status::FinderRemoteInfo {
            name: r.name,
            url: r.fetch_url,
        })
        .collect();

    // Get worktrees
    let worktree_infos = git.list_worktrees(repo_path).unwrap_or_default();
    let mut worktrees = Vec::new();
    let mut all_worktrees_synced = true;

    for wt in &worktree_infos {
        // Skip the main worktree (same as repo_path)
        if wt.path == repo_path {
            continue;
        }
        // Check worktree status
        let wt_synced = if wt.is_bare || wt.is_detached {
            true
        } else {
            git.status(&wt.path)
                .map(|s| s.is_clean_and_synced())
                .unwrap_or(false)
        };
        if !wt_synced {
            all_worktrees_synced = false;
        }
        worktrees.push(FinderWorktreeInfo {
            path: wt.path.clone(),
            branch: wt.branch.clone(),
            synced: wt_synced,
        });
    }

    // Get commit count
    let commit_count = git.commit_count(repo_path).unwrap_or(0);

    // Get stash count
    let stash_count = git.stash_count(repo_path).unwrap_or(0);

    // Check for important ignored files (only if otherwise clean)
    let is_otherwise_clean = repo_status.staged_count == 0
        && repo_status.unstaged_count == 0
        && repo_status.untracked_count == 0
        && repo_status.ahead == 0
        && all_branches_synced
        && all_worktrees_synced;

    let (has_important_ignored_files, important_ignored_files) = if is_otherwise_clean {
        check_important_ignored_files(git, repo_path)
    } else {
        (false, Vec::new())
    };

    // Compute badge
    let badge = compute_badge(
        repo_status.staged_count,
        repo_status.unstaged_count,
        repo_status.untracked_count,
        repo_status.ahead,
        all_branches_synced,
        all_worktrees_synced,
        has_important_ignored_files,
    );

    FinderRepoStatus {
        path: repo_path.to_path_buf(),
        workspace: workspace.map(|s| s.to_string()),
        org: org.map(|s| s.to_string()),
        badge,
        current_branch: repo_status.branch,
        default_branch: None, // Could be determined from remote HEAD
        commit_count,
        staged_count: repo_status.staged_count,
        unstaged_count: repo_status.unstaged_count,
        untracked_count: repo_status.untracked_count,
        ahead: repo_status.ahead,
        behind: repo_status.behind,
        stash_count,
        has_important_ignored_files,
        important_ignored_files,
        branches,
        all_branches_synced,
        remotes,
        worktrees,
        all_worktrees_synced,
    }
}

/// Check if a repo has important ignored files matching the configured patterns.
fn check_important_ignored_files(git: &dyn GitOperations, repo_path: &Path) -> (bool, Vec<String>) {
    let ignored_files = match git.list_ignored_files(repo_path) {
        Ok(files) => files,
        Err(_) => return (false, Vec::new()),
    };

    let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
    let important: Vec<String> = ignored_files
        .into_iter()
        .filter(|f| matches_important_pattern(f, patterns))
        .collect();

    let has_any = !important.is_empty();
    (has_any, important)
}

/// Show daemon status.
fn show_status(ipc_config: &IpcConfig, output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        output.info("Daemon is not running (no status file found)");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            // Check if the PID is still alive
            let pid = status.daemon_pid;
            let is_alive = is_process_alive(pid);

            if is_alive {
                output.info(&format!("Daemon is running (PID: {})", pid));
            } else {
                output.info(&format!("Daemon is not running (stale PID: {})", pid));
            }
            output.info(&format!("Last scan: {}", status.timestamp));
            output.info(&format!("Repos monitored: {}", status.repos.len()));
            output.info(&format!(
                "Workspaces: {}",
                status
                    .workspaces
                    .iter()
                    .map(|w| w.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Err(e) => {
            output.warn(&format!("Could not read status file: {}", e));
        }
    }
    Ok(())
}

/// Stop a running daemon.
fn stop_daemon(ipc_config: &IpcConfig, output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        output.info("No daemon is running");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            let pid = status.daemon_pid;
            if is_process_alive(pid) {
                // Send SIGTERM via kill command
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                output.info(&format!("Sent stop signal to daemon (PID: {})", pid));
            } else {
                output.info("Daemon is not running (stale status file)");
            }
        }
        Err(_) => {
            output.info("Could not read daemon status");
        }
    }
    Ok(())
}

/// Check if a process with the given PID is alive.
fn is_process_alive(pid: u32) -> bool {
    // Use `kill -0` via shell — avoids libc dependency
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;
