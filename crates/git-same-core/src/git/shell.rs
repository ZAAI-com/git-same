//! Shell-based git command implementation.
//!
//! This module provides the real implementation of git operations
//! by invoking git commands through the shell.

use crate::auth::process::run_with_timeout;
use crate::errors::GitError;
use crate::git::traits::{
    BranchInfo, CloneOptions, FetchResult, GitOperations, PullResult, RemoteInfo, RepoStatus,
    WorktreeInfo,
};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, trace};

/// Apple's `/usr/bin/git`: a launcher that looks up the active developer
/// directory on every call before starting the real git.
const APPLE_GIT_LAUNCHER: &str = "/usr/bin/git";

/// How long resolving the real git through `xcrun` may take.
const XCRUN_TIMEOUT: Duration = Duration::from_secs(2);

/// The git program every call runs, resolved once per process.
static GIT_PROGRAM: OnceLock<PathBuf> = OnceLock::new();

/// Shell-based git operations.
///
/// This implementation executes git commands via the shell and parses their output.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShellGit {
    /// Background callers (the monitor) do not emit git trace2 events.
    background: bool,
}

impl ShellGit {
    /// Creates a new ShellGit instance.
    pub fn new() -> Self {
        Self { background: false }
    }

    /// A ShellGit for the background monitor. Its git calls skip trace2
    /// events, so a user's trace2 target (an activity daemon, for example)
    /// sees the user's own git work instead of thousands of status reads per
    /// scan, and each call is cheaper.
    pub fn background() -> Self {
        Self { background: true }
    }

    /// Builds a git command with the environment every call runs under.
    fn command_for(&self, program: &Path, args: &[&str], cwd: Option<&Path>) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // Prevent git from prompting for credentials
        cmd.env("GIT_TERMINAL_PROMPT", "0");

        // Skip optional locks: otherwise status-style commands take
        // .git/index.lock to write back refreshed stat data. The monitor's
        // file watcher sees that write and re-queues the repo it just scanned,
        // and the lock collides with the user's own git commands. Clone,
        // fetch and pull still take the locks they need.
        cmd.env("GIT_OPTIONAL_LOCKS", "0");

        if self.background {
            cmd.env("GIT_TRACE2_EVENT", "0");
        }

        cmd
    }

    /// Runs a git command and returns the output.
    fn run_git(&self, args: &[&str], cwd: Option<&Path>) -> Result<Output, GitError> {
        self.run_git_with(git_program(), args, cwd)
    }

    /// Runs `program`, falling back to `git` from `PATH` when the resolved
    /// program is gone (an Xcode deleted or switched while a long-running
    /// monitor kept the old path).
    fn run_git_with(
        &self,
        program: &Path,
        args: &[&str],
        cwd: Option<&Path>,
    ) -> Result<Output, GitError> {
        let output = match self.command_for(program, args, cwd).output() {
            Err(e) if e.kind() == ErrorKind::NotFound && program != Path::new("git") => {
                debug!(
                    program = %program.display(),
                    "Resolved git is gone; using git from PATH"
                );
                self.command_for(Path::new("git"), args, cwd).output()
            }
            other => other,
        };
        output.map_err(|e| {
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

    /// Parses `git status -b --porcelain` output: branch, ahead and behind
    /// come from the "## " header line, the counts from the file lines.
    fn parse_status_output(&self, output: &str) -> RepoStatus {
        let mut staged_count: usize = 0;
        let mut unstaged_count: usize = 0;
        let mut untracked_count: usize = 0;
        let mut header = "";

        for line in output.lines() {
            // The branch header is not a change. '#' is never a porcelain
            // status code, so no file line can start with it.
            if line.starts_with("## ") {
                if header.is_empty() {
                    header = line;
                }
                continue;
            }
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

        // Format: "## main...origin/main [ahead 1, behind 2]" or "## main"
        let (branch, ahead, behind) = self.parse_branch_info(header);

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

    /// Parses the %(upstream:track) format from for-each-ref.
    /// Format: "[ahead N]", "[behind N]", "[ahead N, behind M]", or "" (synced/no upstream).
    fn parse_track_info(track: &str) -> (u32, u32) {
        let mut ahead = 0;
        let mut behind = 0;
        if let Some(start) = track.find('[') {
            if let Some(end) = track.find(']') {
                let content = &track[start + 1..end];
                for part in content.split(", ") {
                    if let Some(n) = part.strip_prefix("ahead ") {
                        ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix("behind ") {
                        behind = n.parse().unwrap_or(0);
                    }
                }
            }
        }
        (ahead, behind)
    }

    /// Parses a single line of `for-each-ref` output in the format used by
    /// `list_branches`: `<name>\t<upstream:short>\t<upstream:track>`.
    /// Returns `None` if the line has no branch name. Pure function so it
    /// can be unit-tested without shelling out.
    fn parse_branch_line(line: &str) -> Option<BranchInfo> {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        let name = parts.first().unwrap_or(&"").to_string();
        if name.is_empty() {
            return None;
        }

        let upstream_raw = parts.get(1).unwrap_or(&"").to_string();
        let track = parts.get(2).unwrap_or(&"");
        let (ahead, behind) = Self::parse_track_info(track);
        // `for-each-ref` emits "[gone]" in %(upstream:track) when the
        // upstream ref has been deleted. %(upstream:short) still returns
        // the dead ref name, so without this check is_synced would be
        // true for a branch with no reachable upstream.
        let upstream_gone = track.trim() == "[gone]";
        let upstream = if upstream_raw.is_empty() || upstream_gone {
            None
        } else {
            Some(upstream_raw)
        };
        let is_synced = upstream.is_some() && ahead == 0 && behind == 0;

        Some(BranchInfo {
            name,
            upstream,
            ahead,
            behind,
            is_synced,
        })
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

        // Resolve upstream tracking ref and snapshot its commit before fetch.
        let tracking_branch = self
            .run_git_output(
                &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
                Some(repo_path),
            )
            .ok();
        let before_upstream = tracking_branch.as_ref().and_then(|tracking| {
            self.run_git_output(&["rev-parse", tracking], Some(repo_path))
                .ok()
        });

        // Run fetch
        trace!(repo = %repo_path.display(), "Executing fetch --all --prune");
        let output = self.run_git(&["fetch", "--all", "--prune"], Some(repo_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            debug!(repo = %repo_path.display(), error = %stderr, "Fetch failed");
            return Err(GitError::fetch_failed(repo_path, stderr));
        }

        // Compare upstream commit before/after fetch to determine whether remote changed.
        let updated = if let (Some(before_ref), Some(tracking)) =
            (before_upstream, tracking_branch.as_deref())
        {
            let after = self
                .run_git_output(&["rev-parse", tracking], Some(repo_path))
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
                updated: false,
                fast_forward: false,
                error: Some("Working tree has uncommitted changes".to_string()),
            });
        }

        // Try fast-forward only pull
        trace!(repo = %repo_path.display(), "Executing pull --ff-only");
        let output = self.run_git(&["pull", "--ff-only"], Some(repo_path))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let updated = !stdout.contains("Already up to date");
            let fast_forward =
                stdout.contains("Fast-forward") || stdout.contains("Already up to date");

            debug!(
                repo = %repo_path.display(),
                updated,
                fast_forward,
                "Pull completed successfully"
            );

            Ok(PullResult {
                success: true,
                updated,
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
                    updated: false,
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
        // One call yields the branch header and the file lines. The header
        // comes first, so trimming the output never strips the leading space
        // of a file line (" M" is unstaged, "M " is staged).
        let output = self.run_git_output(&["status", "-b", "--porcelain"], Some(repo_path))?;

        Ok(self.parse_status_output(&output))
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

    fn list_branches(&self, repo_path: &Path) -> Result<Vec<BranchInfo>, GitError> {
        // Use for-each-ref to get branch name, upstream, and tracking status in one call
        let output = self.run_git_output(
            &[
                "for-each-ref",
                "--format=%(refname:short)\t%(upstream:short)\t%(upstream:track)",
                "refs/heads/",
            ],
            Some(repo_path),
        )?;

        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(Self::parse_branch_line)
            .collect())
    }

    fn list_remotes(&self, repo_path: &Path) -> Result<Vec<RemoteInfo>, GitError> {
        let output = self.run_git_output(&["remote", "-v"], Some(repo_path))?;
        if output.is_empty() {
            return Ok(Vec::new());
        }

        // git remote -v outputs pairs: "origin\turl (fetch)" and "origin\turl (push)"
        // Collect into a map keyed by remote name
        let mut remotes: std::collections::BTreeMap<String, (String, String)> =
            std::collections::BTreeMap::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let name = parts[0].to_string();
            let url = parts[1].to_string();
            let kind = parts[2]; // "(fetch)" or "(push)"

            let entry = remotes
                .entry(name)
                .or_insert_with(|| (String::new(), String::new()));
            if kind == "(fetch)" {
                entry.0 = url;
            } else if kind == "(push)" {
                entry.1 = url;
            }
        }

        Ok(remotes
            .into_iter()
            .map(|(name, (fetch_url, push_url))| RemoteInfo {
                name,
                fetch_url: fetch_url.clone(),
                push_url: if push_url.is_empty() {
                    fetch_url
                } else {
                    push_url
                },
            })
            .collect())
    }

    fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<WorktreeInfo>, GitError> {
        let output = self.run_git_output(&["worktree", "list", "--porcelain"], Some(repo_path))?;
        if output.is_empty() {
            return Ok(Vec::new());
        }

        // Porcelain format: blocks separated by blank lines
        // Each block: "worktree <path>\nHEAD <sha>\nbranch refs/heads/<name>\n"
        // Or: "worktree <path>\nHEAD <sha>\ndetached\n"
        // Or: "worktree <path>\nbare\n"
        let mut worktrees = Vec::new();
        let mut current_path: Option<std::path::PathBuf> = None;
        let mut current_branch: Option<String> = None;
        let mut is_bare = false;
        let mut is_detached = false;

        for line in output.lines() {
            if line.is_empty() {
                // End of a worktree block
                if let Some(path) = current_path.take() {
                    worktrees.push(WorktreeInfo {
                        path,
                        branch: current_branch.take(),
                        is_bare,
                        is_detached,
                    });
                }
                is_bare = false;
                is_detached = false;
                continue;
            }

            if let Some(path_str) = line.strip_prefix("worktree ") {
                current_path = Some(std::path::PathBuf::from(path_str));
            } else if let Some(branch_ref) = line.strip_prefix("branch ") {
                // Strip "refs/heads/" prefix to get short branch name
                current_branch = Some(
                    branch_ref
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch_ref)
                        .to_string(),
                );
            } else if line == "bare" {
                is_bare = true;
            } else if line == "detached" {
                is_detached = true;
            }
        }

        // Handle last block (porcelain output may not end with blank line)
        if let Some(path) = current_path.take() {
            worktrees.push(WorktreeInfo {
                path,
                branch: current_branch.take(),
                is_bare,
                is_detached,
            });
        }

        Ok(worktrees)
    }

    fn commit_count(&self, repo_path: &Path) -> Result<u64, GitError> {
        let output = self.run_git_output(&["rev-list", "--count", "HEAD"], Some(repo_path))?;
        output.parse().map_err(|_| {
            GitError::command_failed(
                "git rev-list --count HEAD",
                format!("Could not parse commit count: '{}'", output),
            )
        })
    }

    fn stash_count(&self, repo_path: &Path) -> Result<usize, GitError> {
        let output = self.run_git_output(&["stash", "list"], Some(repo_path));
        match output {
            Ok(s) if s.is_empty() => Ok(0),
            Ok(s) => Ok(s.lines().count()),
            // git stash list returns error on repos with no stashes in some git versions
            Err(_) => Ok(0),
        }
    }

    fn list_ignored_files(
        &self,
        repo_path: &Path,
        skip_dirs: &[String],
    ) -> Result<Vec<String>, GitError> {
        // Fully ignored folders come back once as `dir/` instead of every
        // file inside them (astro: 1,137 entries instead of 102,106).
        let listing = self.run_git_nul_separated(
            &[
                "ls-files",
                "-z",
                "--others",
                "--ignored",
                "--exclude-standard",
                "--directory",
            ],
            repo_path,
        )?;
        let (mut files, dirs) = split_ignored_entries(&listing, skip_dirs);

        // Look inside the collapsed folders that are not dependency folders,
        // all in one call, so a `config/.env` in an ignored `config/` is
        // still found.
        if !dirs.is_empty() {
            let specs: Vec<String> = dirs.iter().map(|d| format!(":(literal){d}")).collect();
            let mut args = vec![
                "ls-files",
                "-z",
                "--others",
                "--ignored",
                "--exclude-standard",
                "--",
            ];
            args.extend(specs.iter().map(String::as_str));
            let inside = self.run_git_nul_separated(&args, repo_path)?;
            files.extend(split_ignored_entries(&inside, skip_dirs).0);
        }

        // An untracked folder holding only ignored files is listed both as
        // `dir/` and as `dir/file`, so the second call can repeat a file.
        let mut seen = HashSet::new();
        files.retain(|file| seen.insert(file.clone()));
        Ok(files)
    }
}

impl ShellGit {
    /// Runs a `-z` git command and returns stdout untrimmed: with NUL
    /// separators, trimming could eat a leading space of the first path.
    fn run_git_nul_separated(&self, args: &[&str], repo_path: &Path) -> Result<String, GitError> {
        let output = self.run_git(args, Some(repo_path))?;
        if !output.status.success() {
            return Err(GitError::command_failed(
                format!("git {}", args.join(" ")),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Splits NUL-separated `ls-files --directory` output into ignored files and
/// collapsed folders (`dir/`), dropping every entry that lies in, or is, a
/// folder named in `skip_dirs` (dependency folders such as `node_modules`).
fn split_ignored_entries(listing: &str, skip_dirs: &[String]) -> (Vec<String>, Vec<String>) {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in listing.split('\0').filter(|entry| !entry.is_empty()) {
        let in_skipped_dir = entry
            .trim_end_matches('/')
            .split('/')
            .any(|component| skip_dirs.iter().any(|skip| skip == component));
        if in_skipped_dir {
            continue;
        }
        if entry.ends_with('/') {
            dirs.push(entry.to_string());
        } else {
            files.push(entry.to_string());
        }
    }
    (files, dirs)
}

/// The git program to run, resolved on first use.
fn git_program() -> &'static Path {
    GIT_PROGRAM.get_or_init(|| {
        let program = choose_git_program(
            cfg!(target_os = "macos"),
            find_on_path("git"),
            std::env::var_os("DEVELOPER_DIR").is_some(),
            xcrun_find_git,
        );
        debug!(program = %program.display(), "Resolved git program");
        program
    })
}

/// Picks the git binary to run.
///
/// Only Apple's launcher is replaced: on macOS, when the `git` on `PATH` is
/// `/usr/bin/git`, the real binary it would start is used directly, which
/// saves the launcher's lookup on every call (2,000+ calls per monitor scan).
/// A git the user installed (Homebrew, for example) and a per-shell
/// `DEVELOPER_DIR` keep working through plain `git`, as does any failure.
fn choose_git_program(
    is_macos: bool,
    path_git: Option<PathBuf>,
    developer_dir_set: bool,
    xcrun_git: impl FnOnce() -> Option<PathBuf>,
) -> PathBuf {
    let plain = PathBuf::from("git");
    if !is_macos || developer_dir_set {
        return plain;
    }
    if path_git.as_deref() != Some(Path::new(APPLE_GIT_LAUNCHER)) {
        return plain;
    }
    match xcrun_git() {
        Some(real) if real.is_file() => real,
        _ => plain,
    }
}

/// The first `name` on `PATH` that is a file.
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Asks Xcode which git binary `/usr/bin/git` would start.
fn xcrun_find_git() -> Option<PathBuf> {
    let output = run_with_timeout("/usr/bin/xcrun", &["--find", "git"], XCRUN_TIMEOUT).ok()?;
    if !output.status.success() {
        return None;
    }
    let found = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!found.is_empty()).then(|| PathBuf::from(found))
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
