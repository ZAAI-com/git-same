//! Shell-based git command implementation.
//!
//! This module provides the real implementation of git operations
//! by invoking git commands through the shell.

use crate::errors::GitError;
use crate::git::traits::{CloneOptions, FetchResult, GitOperations, PullResult, RepoStatus};
use std::path::Path;
use std::process::{Command, Output};
use tracing::{debug, trace};

/// Shell-based git operations.
///
/// This implementation executes git commands via the shell and parses their output.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShellGit;

impl ShellGit {
    /// Creates a new ShellGit instance.
    pub fn new() -> Self {
        Self
    }

    /// Runs a git command and returns the output.
    fn run_git(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        let mut cmd = Command::new("git");
        cmd.args(args);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // Prevent git from prompting for credentials
        cmd.env("GIT_TERMINAL_PROMPT", "0");

        cmd.output().map_err(|e| {
            GitError::command_failed(
                format!("git {}", args.join(" ")),
                format!("Failed to execute: {}", e),
            )
        })
    }

    /// Runs a git command and returns stdout as a string.
    fn run_git_output(&self, args: &[&str], cwd: Option<&Path>) -> Result<String, GitError> {
        let output = self.run_git(args, cwd)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(GitError::command_failed(
                format!("git {}", args.join(" ")),
                stderr,
            ))
        }
    }

    /// Checks if a git command succeeds.
    fn run_git_check(&self, args: &[&str], cwd: Option<&Path>) -> bool {
        self.run_git(args, cwd)
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Parses the porcelain status output.
    fn parse_status_output(&self, output: &str, branch_output: &str) -> RepoStatus {
        let mut staged_count: usize = 0;
        let mut unstaged_count: usize = 0;
        let mut untracked_count: usize = 0;

        for line in output.lines() {
            if line.len() < 2 {
                continue;
            }
            let bytes = line.as_bytes();
            let x = bytes[0]; // index (staged) status
            let y = bytes[1]; // working tree (unstaged) status

            if x == b'?' && y == b'?' {
                untracked_count += 1;
            } else {
                if x != b' ' && x != b'?' {
                    staged_count += 1;
                }
                if y != b' ' && y != b'?' {
                    unstaged_count += 1;
                }
            }
        }
        let is_uncommitted = staged_count > 0 || unstaged_count > 0;
        let has_untracked = untracked_count > 0;

        // Parse branch info from `git status -b --porcelain`
        // Format: "## main...origin/main [ahead 1, behind 2]" or "## main"
        let (branch, ahead, behind) = self.parse_branch_info(branch_output);

        RepoStatus {
            branch,
            is_uncommitted,
            ahead,
            behind,
            has_untracked,
            staged_count,
            unstaged_count,
            untracked_count,
        }
    }

    /// Parses branch info from git status -b --porcelain output.
    fn parse_branch_info(&self, output: &str) -> (String, u32, u32) {
        let first_line = output.lines().next().unwrap_or("");

        // Remove the "## " prefix
        let line = first_line.strip_prefix("## ").unwrap_or(first_line);

        // Split on "..." to get branch name and tracking info
        let (branch_part, info_part): (&str, Option<&str>) = if let Some(idx) = line.find("...") {
            (&line[..idx], Some(&line[idx + 3..]))
        } else {
            // No tracking branch, but might have [ahead X, behind Y] directly
            // e.g., "## feature [ahead 1, behind 2]"
            if let Some(bracket_idx) = line.find('[') {
                (line[..bracket_idx].trim_end(), Some(&line[bracket_idx..]))
            } else {
                let branch = line.split_whitespace().next().unwrap_or("HEAD");
                (branch, None)
            }
        };

        let branch = branch_part.to_string();
        let mut ahead = 0;
        let mut behind = 0;

        // Parse ahead/behind from info part
        // Format: "origin/main [ahead 1, behind 2]" or "[ahead 1]" or "origin/main [ahead 1]"
        if let Some(info) = info_part {
            if let Some(start) = info.find('[') {
                if let Some(end) = info.find(']') {
                    let bracket_content = &info[start + 1..end];
                    for part in bracket_content.split(", ") {
                        if let Some(n) = part.strip_prefix("ahead ") {
                            ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix("behind ") {
                            behind = n.parse().unwrap_or(0);
                        }
                    }
                }
            }
        }

        (branch, ahead, behind)
    }
}

impl GitOperations for ShellGit {
    fn clone_repo(&self, url: &str, target: &Path, options: &CloneOptions) -> Result<(), GitError> {
        debug!(
            url,
            target = %target.display(),
            depth = options.depth,
            branch = options.branch.as_deref().unwrap_or("default"),
            recurse_submodules = options.recurse_submodules,
            "Starting git clone"
        );

        let mut args = vec!["clone"];

        // Add depth if specified
        let depth_str;
        if options.depth > 0 {
            depth_str = options.depth.to_string();
            args.push("--depth");
            args.push(&depth_str);
        }

        // Add branch if specified
        if let Some(ref branch) = options.branch {
            args.push("--branch");
            args.push(branch);
        }

        // Add submodule recursion if requested
        if options.recurse_submodules {
            args.push("--recurse-submodules");
        }

        // Add URL and target
        args.push(url);
        let target_str = target.to_string_lossy();
        args.push(&target_str);

        trace!(args = ?args, "Executing git command");
        let output = self.run_git(&args, None)?;

        if output.status.success() {
            debug!(url, target = %target.display(), "Clone completed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            debug!(url, error = %stderr, "Clone failed");
            Err(GitError::clone_failed(url, stderr))
        }
    }

    fn fetch(&self, repo_path: &Path) -> Result<FetchResult, GitError> {
        debug!(repo = %repo_path.display(), "Starting git fetch");

        // Get current HEAD before fetch
        let before = self
            .run_git_output(&["rev-parse", "HEAD"], Some(repo_path))
            .ok();

        // Run fetch
        trace!(repo = %repo_path.display(), "Executing fetch --all --prune");
        let output = self.run_git(&["fetch", "--all", "--prune"], Some(repo_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            debug!(repo = %repo_path.display(), error = %stderr, "Fetch failed");
            return Err(GitError::fetch_failed(repo_path, stderr));
        }

        // Check if remote tracking branch has new commits
        let tracking_branch = self
            .run_git_output(
                &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
                Some(repo_path),
            )
            .ok();

        let updated = if let (Some(before_ref), Some(tracking)) = (before, tracking_branch) {
            let after = self
                .run_git_output(&["rev-parse", &tracking], Some(repo_path))
                .ok();
            after.map(|a| a != before_ref).unwrap_or(false)
        } else {
            false
        };

        // Count new commits if updated
        let new_commits = if updated {
            self.run_git_output(&["rev-list", "--count", "HEAD..@{u}"], Some(repo_path))
                .ok()
                .and_then(|s| s.parse().ok())
        } else {
            Some(0)
        };

        debug!(
            repo = %repo_path.display(),
            updated,
            new_commits = new_commits.unwrap_or(0),
            "Fetch completed"
        );

        Ok(FetchResult {
            updated,
            new_commits,
        })
    }

    fn pull(&self, repo_path: &Path) -> Result<PullResult, GitError> {
        debug!(repo = %repo_path.display(), "Starting git pull");

        // First check status
        let status = self.status(repo_path)?;

        if status.is_uncommitted {
            debug!(repo = %repo_path.display(), "Skipping pull: uncommitted changes");
            return Ok(PullResult {
                success: false,
                fast_forward: false,
                error: Some("Working tree has uncommitted changes".to_string()),
            });
        }

        // Try fast-forward only pull
        trace!(repo = %repo_path.display(), "Executing pull --ff-only");
        let output = self.run_git(&["pull", "--ff-only"], Some(repo_path))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let fast_forward =
                stdout.contains("Fast-forward") || stdout.contains("Already up to date");

            debug!(repo = %repo_path.display(), fast_forward, "Pull completed successfully");

            Ok(PullResult {
                success: true,
                fast_forward,
                error: None,
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Check if it's a non-fast-forward situation
            if stderr.contains("Not possible to fast-forward") {
                debug!(repo = %repo_path.display(), "Pull failed: branch has diverged");
                Ok(PullResult {
                    success: false,
                    fast_forward: false,
                    error: Some("Cannot fast-forward, local branch has diverged".to_string()),
                })
            } else {
                debug!(repo = %repo_path.display(), error = %stderr, "Pull failed");
                Err(GitError::pull_failed(repo_path, stderr))
            }
        }
    }

    fn status(&self, repo_path: &Path) -> Result<RepoStatus, GitError> {
        // Get status with branch info
        let branch_output =
            self.run_git_output(&["status", "-b", "--porcelain"], Some(repo_path))?;

        // Get just the file status
        let status_output = self.run_git_output(&["status", "--porcelain"], Some(repo_path))?;

        Ok(self.parse_status_output(&status_output, &branch_output))
    }

    fn is_repo(&self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        self.run_git_check(&["rev-parse", "--git-dir"], Some(path))
    }

    fn current_branch(&self, repo_path: &Path) -> Result<String, GitError> {
        self.run_git_output(&["rev-parse", "--abbrev-ref", "HEAD"], Some(repo_path))
    }

    fn remote_url(&self, repo_path: &Path, remote: &str) -> Result<String, GitError> {
        self.run_git_output(&["remote", "get-url", remote], Some(repo_path))
    }

    fn recent_commits(&self, repo_path: &Path, limit: usize) -> Result<Vec<String>, GitError> {
        let limit_arg = format!("-{}", limit);
        let output = self.run_git_output(&["log", "--oneline", &limit_arg], Some(repo_path))?;
        if output.is_empty() {
            return Ok(Vec::new());
        }
        Ok(output.lines().map(|l| l.to_string()).collect())
    }
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
