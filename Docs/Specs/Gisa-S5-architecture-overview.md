# Gisa Architecture Overview

## Quick Start (TL;DR)

```bash
# 1. Create project
cargo new gisa && cd gisa

# 2. Copy Cargo.toml from Task 0.2 (dependencies section)

# 3. Create file structure from Task 0.3:
mkdir -p src/{config,auth,discovery,clone,sync}
touch src/{lib,cli,types}.rs
touch src/config/{mod,parser}.rs
touch src/auth/{mod,gh_cli}.rs
touch src/discovery/{mod,github}.rs
touch src/clone/{mod,parallel}.rs
touch src/sync/{mod,manager}.rs

# 4. Work through Phases 1-7 in order, copy-pasting code

# 5. Test with:
cargo run -- clone ~/test --dry-run
```

**Scope:** ~2000 lines of Rust across 14 files
**Time estimate:** Varies by experience
**Prerequisites:** Rust installed, GitHub CLI authenticated

---

## System Overview

Gisa is a CLI tool that mirrors GitHub organization and repository structures to the local filesystem. It discovers all orgs/repos a user has access to and clones them with configurable options.

```
┌─────────────────────────────────────────────────────────────────┐
│                         Gisa CLI                                │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Config    │  │  Auth       │  │  CLI        │             │
│  │   Manager   │  │  Manager    │  │  Interface  │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          │                                      │
│                    ┌─────▼─────┐                                │
│                    │   Core    │                                │
│                    │  Engine   │                                │
│                    └─────┬─────┘                                │
│                          │                                      │
│         ┌────────────────┼────────────────┐                     │
│         │                │                │                     │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐             │
│  │  Discovery  │  │   Clone     │  │   Sync      │             │
│  │   Module    │  │   Manager   │  │   Manager   │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
          ▼                ▼                ▼
   ┌────────────┐   ┌────────────┐   ┌────────────┐
   │  GitHub    │   │  Local     │   │  Git       │
   │  API       │   │  Filesystem│   │  Operations│
   └────────────┘   └────────────┘   └────────────┘
```

## Core Components

### 1. CLI Interface
- Command parsing and validation
- Progress bars and output formatting
- Interactive prompts for missing config
- Dry-run mode display

### 2. Config Manager
- Loads `gisa.config.toml` from project directory
- TOML configuration format (Rust ecosystem standard)
- Validates and merges CLI flags with config file
- Stores: base path, clone options, concurrency, sync behavior, filters

### 3. Auth Manager
- **Primary**: GitHub CLI (`gh auth token`) integration
- **Fallback 1**: SSH key detection and validation
- **Fallback 2**: Personal Access Token from env/config
- Token validation before operations begin

### 4. Discovery Module
- Fetches all orgs user belongs to via GitHub API
- Fetches all repos per org (handles pagination)
- Fetches user's personal repos
- Returns unified list with metadata (visibility, clone URLs, archived status)

### 5. Clone Manager
- Parallel cloning with configurable concurrency (default: 4)
- SSH clone URL preferred, HTTPS fallback
- Supports clone options: `--depth`, `--branch`, `--recurse-submodules`
- Creates directory structure: `<base>/<org>/<repo>/`

### 6. Sync Manager
- Detects existing clones
- Configurable behavior: `fetch` (safe) or `pull` (updates working tree)
- Reports conflicts/uncommitted changes without modifying
- Tracks new repos added to orgs since last sync

## Data Flow

```
1. User runs: gisa sync ~/github

2. Auth Manager
   └─→ Obtains GitHub token (gh CLI → SSH → PAT)

3. Discovery Module
   └─→ GET /user/orgs → List of orgs
   └─→ GET /orgs/{org}/repos → Repos per org (paginated)
   └─→ GET /user/repos → Personal repos

4. Core Engine
   └─→ Compares discovered repos with local filesystem
   └─→ Generates action plan: [clone: 12, sync: 45, skip: 3]

5. Dry-run check
   └─→ If --dry-run: display plan and exit

6. Clone/Sync Manager (parallel)
   └─→ Clone new repos (SSH preferred)
   └─→ Sync existing repos (fetch or pull)
   └─→ Report failures at end

7. Output
   └─→ Summary: cloned 12, synced 45, failed 2
   └─→ Failed repos listed with error reasons
```

## Directory Structure

### Default Structure
```
~/github/                    # Base path (configurable)
├── my-org/                  # Organization
│   ├── repo-one/
│   ├── repo-two/
│   └── repo-three/
├── another-org/
│   └── their-repo/
└── octocat/                 # User's personal repos (GitHub username)
    ├── my-project/
    └── dotfiles/
```

### Configurable via `gisa.config.toml`
```toml
base_path = "~/github"
structure = "{org}/{repo}"  # Default
# Alternative: "{org}-{repo}" for flat structure
```

## Error Handling Strategy

| Scenario | Behavior |
| --- | --- |
| Auth failure | Stop, display auth instructions |
| API rate limit | Pause, retry with backoff |
| Single repo clone fails | Log error, continue with others |
| Network timeout | Retry 3x, then skip and log |
| Repo exists with changes | Skip sync, warn user |
| Permission denied (private repo) | Skip, log (user may have lost access) |

At completion: display summary with all failures and reasons.

## Distribution

| Priority | Method | Command | Target Audience |
| --- | --- | --- | --- |
| 1 | Homebrew | `brew install gisa` | macOS users (primary) |
| 2 | GitHub Releases | Download binary | All platforms, no toolchain needed |
| 3 | Cargo | `cargo install gisa` | Rust developers |

### Homebrew (Primary)

```bash
brew install gisa
```

Homebrew formula maintained in homebrew-core or custom tap.

### GitHub Releases

Pre-built binaries for each release:
- `gisa-x86_64-apple-darwin` (macOS Intel)
- `gisa-aarch64-apple-darwin` (macOS Apple Silicon)
- `gisa-x86_64-unknown-linux-gnu` (Linux)
- `gisa-x86_64-pc-windows-msvc.exe` (Windows)

### Cargo (Rust developers)

```bash
cargo install gisa
```

Builds from source via crates.io. Requires Rust toolchain.

## CLI Command Naming

Commands follow standard git naming conventions for familiarity:

| Gisa Command | Git Equivalent | Description |
| --- | --- | --- |
| `gisa clone` | `git clone` | Clone all repos |
| `gisa fetch` | `git fetch` | Fetch updates (safe, no working tree changes) |
| `gisa pull` | `git pull` | Pull updates (modifies working tree) |
| `gisa status` | `git status` | Show sync status of all repos |
| `gisa init` | `git init` | Initialize config file |

## Code Organization

### Colocated Documentation

Each module includes its own README for discoverability:

```
src/
├── auth/
│   ├── mod.rs
│   ├── gh_cli.rs
│   ├── gh_cli.test.rs      # Colocated test
│   └── README.md           # Auth module docs
├── discovery/
│   ├── mod.rs
│   ├── github.rs
│   ├── github.test.rs      # Colocated test
│   └── README.md           # Discovery module docs
├── clone/
│   ├── mod.rs
│   ├── parallel.rs
│   ├── parallel.test.rs    # Colocated test
│   └── README.md           # Clone module docs
└── README.md               # Root src docs
```

### Colocated Tests

Tests live next to the code they test using the `.test.rs` suffix:

- `auth/gh_cli.rs` → `auth/gh_cli.test.rs`
- `discovery/github.rs` → `discovery/github.test.rs`
- `config/parser.rs` → `config/parser.test.rs`

Benefits:
- Easy to find tests for any module
- Tests stay in sync with implementation
- Clear ownership of test coverage

## State Management

### V1: File-Based Cache

No database required. State is managed via simple files:

```
~/.config/gisa/              # XDG config directory
└── gisa.cache.json          # Discovery cache (auto-generated)

~/github/                    # Base path
└── gisa.config.toml         # User config (project-level)
```

**Cache file** (`gisa.cache.json`):
```json
{
  "last_discovery": "2024-01-15T10:30:00Z",
  "username": "octocat",
  "orgs": ["my-org", "another-org"],
  "repos": [
    {
      "full_name": "my-org/repo-one",
      "ssh_url": "git@github.com:my-org/repo-one.git",
      "pushed_at": "2024-01-14T08:00:00Z"
    }
  ]
}
```

**Cache behavior:**
- Invalidated after 1 hour (configurable)
- Force refresh with `--refresh` flag
- Used to detect new repos without full API scan
- Stores `pushed_at` for incremental sync detection

### V2+: SQLite (Future)

SQLite may be added if these features become requirements:
- Sync history tracking ("what changed last week?")
- Per-repo metadata (custom tags, notes)
- Offline mode with full local state
- Query interface for repo management

## Future Extensibility

The architecture supports planned features:

- **V2: Filters** — Discovery module accepts filter predicates
- **V2: Single org** — Discovery module accepts org parameter
- **V3: GitHub Enterprise** — Auth/Discovery modules accept base URL
- **V4: GitLab/Bitbucket** — Abstract Discovery/Clone behind provider interface

```
┌─────────────────────────────────────────┐
│           Provider Interface            │
├─────────────────────────────────────────┤
│  + authenticate()                       │
│  + discoverOrgs()                       │
│  + discoverRepos(org)                   │
│  + getCloneUrl(repo, protocol)          │
└─────────────────────────────────────────┘
         ▲           ▲           ▲
         │           │           │
    ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
    │ GitHub  │ │ GitLab  │ │Bitbucket│
    │Provider │ │Provider │ │Provider │
    └─────────┘ └─────────┘ └─────────┘
```

---

# Implementation Plan

This plan breaks Gisa into small, testable tasks. Each task has clear inputs, outputs, and acceptance criteria.

---

## Phase 0: Project Setup

### Task 0.1: Create Rust Project

**What to do:**
```bash
cargo new gisa
cd gisa
```

**Files created:**
- `Cargo.toml`
- `src/main.rs`

**Done when:** `cargo build` succeeds.

---

### Task 0.2: Add Dependencies to Cargo.toml

**Replace \****`Cargo.toml`**\*\* with:**
```toml
[package]
name = "gisa"
version = "0.1.0"
edition = "2021"
description = "Mirror GitHub org/repo structure locally"
license = "MIT"

[dependencies]
# CLI parsing
clap = { version = "4", features = ["derive"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client for GitHub API
reqwest = { version = "0.12", features = ["json"] }

# JSON/TOML serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Progress bars and terminal output
indicatif = "0.17"
console = "0.15"

# XDG directories (~/.config/gisa)
directories = "5"

# Error handling
thiserror = "1"
anyhow = "1"

# Shell expansion (~/ paths)
shellexpand = "3"

[dev-dependencies]
# Testing
tokio-test = "0.4"
mockito = "1"
tempfile = "3"
```

**Done when:** `cargo build` succeeds with all dependencies.

---

### Task 0.3: Create Module Structure

**Create these empty files:**
```
src/
├── main.rs           # Entry point
├── lib.rs            # Library root (re-exports modules)
├── cli.rs            # CLI argument parsing
├── config/
│   ├── mod.rs        # Config module root
│   └── parser.rs     # TOML parsing
├── auth/
│   ├── mod.rs        # Auth module root
│   └── gh_cli.rs     # GitHub CLI integration
├── discovery/
│   ├── mod.rs        # Discovery module root
│   └── github.rs     # GitHub API calls
├── clone/
│   ├── mod.rs        # Clone module root
│   └── parallel.rs   # Parallel cloning
├── sync/
│   ├── mod.rs        # Sync module root
│   └── manager.rs    # Sync logic
└── types.rs          # Shared types (Repo, Org, etc.)
```

**For each \****`mod.rs`**\*\* file, add:**
```rust
// mod.rs template - replace with actual module name
pub mod parser;  // or gh_cli, github, parallel, manager
```

**For \****`lib.rs`**\*\*:**
```rust
pub mod cli;
pub mod config;
pub mod auth;
pub mod discovery;
pub mod clone;
pub mod sync;
pub mod types;
```

**Done when:** `cargo check` passes with no errors.

---

## Phase 1: Types and Config

### Task 1.1: Define Core Types

**File:** `src/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// A GitHub organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Org {
    pub login: String,
    pub id: u64,
}

/// A GitHub repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub name: String,
    pub full_name: String,          // "org/repo"
    pub ssh_url: String,            // "git@github.com:org/repo.git"
    pub clone_url: String,          // "https://github.com/org/repo.git"
    pub default_branch: String,
    pub private: bool,
    pub archived: bool,
    pub fork: bool,
}

/// Which organization or user owns a repo
#[derive(Debug, Clone)]
pub struct OwnedRepo {
    pub owner: String,              // Org name or username
    pub repo: Repo,
}

/// Result of comparing discovered repos with local filesystem
#[derive(Debug)]
pub struct ActionPlan {
    pub to_clone: Vec<OwnedRepo>,   // New repos to clone
    pub to_sync: Vec<OwnedRepo>,    // Existing repos to sync
    pub skipped: Vec<OwnedRepo>,    // Repos skipped (dirty, conflicts)
}

/// Outcome of a clone or sync operation
#[derive(Debug)]
pub enum OpResult {
    Success,
    Failed(String),                 // Error message
    Skipped(String),                // Reason for skipping
}
```

**Done when:** `cargo check` passes.

---

### Task 1.2: Implement Config Parser

**File:** `src/config/parser.rs`

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Clone-specific options
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CloneOptions {
    #[serde(default)]
    pub depth: u32,                 // 0 = full clone

    #[serde(default)]
    pub branch: String,             // Empty = default branch

    #[serde(default)]
    pub recurse_submodules: bool,
}

/// Filter options
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterOptions {
    #[serde(default)]
    pub include_archived: bool,

    #[serde(default)]
    pub include_forks: bool,
}

/// Full configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_base_path")]
    pub base_path: String,

    #[serde(default = "default_structure")]
    pub structure: String,

    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    #[serde(default = "default_sync_mode")]
    pub sync_mode: String,

    #[serde(default)]
    pub clone: CloneOptions,

    #[serde(default)]
    pub filters: FilterOptions,
}

fn default_base_path() -> String { "~/github".to_string() }
fn default_structure() -> String { "{org}/{repo}".to_string() }
fn default_concurrency() -> usize { 4 }
fn default_sync_mode() -> String { "fetch".to_string() }

impl Default for Config {
    fn default() -> Self {
        Config {
            base_path: default_base_path(),
            structure: default_structure(),
            concurrency: default_concurrency(),
            sync_mode: default_sync_mode(),
            clone: CloneOptions::default(),
            filters: FilterOptions::default(),
        }
    }
}

impl Config {
    /// Load config from file, or return defaults if file doesn't exist
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read config: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| "Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Expand ~ in base_path to actual home directory
    pub fn expanded_base_path(&self) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(&self.base_path);
        Ok(PathBuf::from(expanded.as_ref()))
    }

    /// Generate repo path from structure pattern
    pub fn repo_path(&self, org: &str, repo: &str) -> Result<PathBuf> {
        let base = self.expanded_base_path()?;
        let relative = self.structure
            .replace("{org}", org)
            .replace("{repo}", repo);
        Ok(base.join(relative))
    }
}
```

**File:** `src/config/mod.rs`
```rust
pub mod parser;
pub use parser::Config;
```

**Test manually:**
Create a file `test.toml`:
```toml
base_path = "~/test"
concurrency = 8
```

Add temporary test code to `main.rs`:
```rust
use std::path::Path;
mod config;

fn main() {
    let cfg = config::Config::load(Path::new("test.toml")).unwrap();
    println!("{:?}", cfg);
}
```

**Done when:** Running `cargo run` prints the config with `base_path = "~/test"` and `concurrency = 8`.

---

### Task 1.3: Write Config Tests

**File:** `src/config/parser.test.rs`

```rust
use super::*;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.base_path, "~/github");
    assert_eq!(config.concurrency, 4);
    assert_eq!(config.sync_mode, "fetch");
    assert!(!config.filters.include_archived);
}

#[test]
fn test_load_minimal_config() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "base_path = \"~/custom\"").unwrap();

    let config = Config::load(file.path()).unwrap();
    assert_eq!(config.base_path, "~/custom");
    assert_eq!(config.concurrency, 4); // Default preserved
}

#[test]
fn test_load_full_config() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"
base_path = "~/repos"
concurrency = 8
sync_mode = "pull"

[clone]
depth = 1
recurse_submodules = true

[filters]
include_archived = true
include_forks = true
"#).unwrap();

    let config = Config::load(file.path()).unwrap();
    assert_eq!(config.base_path, "~/repos");
    assert_eq!(config.concurrency, 8);
    assert_eq!(config.clone.depth, 1);
    assert!(config.clone.recurse_submodules);
    assert!(config.filters.include_archived);
}

#[test]
fn test_missing_file_returns_defaults() {
    let config = Config::load(Path::new("/nonexistent/config.toml")).unwrap();
    assert_eq!(config.base_path, "~/github");
}

#[test]
fn test_repo_path_generation() {
    let config = Config {
        base_path: "/home/user/github".to_string(),
        structure: "{org}/{repo}".to_string(),
        ..Config::default()
    };

    let path = config.repo_path("my-org", "my-repo").unwrap();
    assert_eq!(path, PathBuf::from("/home/user/github/my-org/my-repo"));
}
```

**Add to \****`src/config/mod.rs`**\*\*:**
```rust
#[cfg(test)]
mod parser_test;
```

**Done when:** `cargo test config` passes all tests.

---

## Phase 2: CLI Interface

### Task 2.1: Implement CLI Parser

**File:** `src/cli.rs`

```rust
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gisa")]
#[command(version)]
#[command(about = "Mirror GitHub org/repo structure locally")]
#[command(long_about = "Gisa discovers all GitHub organizations and repositories you have access to, then clones them to your local filesystem maintaining the org/repo directory structure.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Clone all repos from your GitHub organizations
    Clone {
        /// Base directory for cloned repos (overrides config)
        #[arg(default_value = "~/github")]
        path: String,

        /// Number of parallel clone operations
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Preview what would be cloned without actually cloning
        #[arg(long)]
        dry_run: bool,

        /// Shallow clone with specified depth
        #[arg(long)]
        depth: Option<u32>,

        /// Clone submodules
        #[arg(long)]
        recurse_submodules: bool,

        /// Include archived repositories
        #[arg(long)]
        include_archived: bool,

        /// Include forked repositories
        #[arg(long)]
        include_forks: bool,

        /// Force re-discovery (ignore cache)
        #[arg(long)]
        refresh: bool,
    },

    /// Fetch updates for all cloned repos (safe, no working tree changes)
    Fetch {
        /// Base directory containing cloned repos
        #[arg(default_value = "~/github")]
        path: String,

        /// Number of parallel operations
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Preview what would be fetched
        #[arg(long)]
        dry_run: bool,
    },

    /// Pull updates for all cloned repos (modifies working tree)
    Pull {
        /// Base directory containing cloned repos
        #[arg(default_value = "~/github")]
        path: String,

        /// Number of parallel operations
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Preview what would be pulled
        #[arg(long)]
        dry_run: bool,
    },

    /// Show sync status of all repos
    Status {
        /// Base directory containing cloned repos
        #[arg(default_value = "~/github")]
        path: String,
    },

    /// Initialize a new gisa.config.toml file
    Init {
        /// Directory to create config in
        #[arg(default_value = ".")]
        path: String,
    },
}

/// Parse command line arguments
pub fn parse() -> Cli {
    Cli::parse()
}
```

**Update \****`src/main.rs`**\*\*:**
```rust
mod cli;

fn main() {
    let args = cli::parse();

    match args.command {
        cli::Commands::Clone { path, dry_run, .. } => {
            println!("Would clone to: {}", path);
            if dry_run {
                println!("(dry run mode)");
            }
        }
        cli::Commands::Fetch { path, .. } => {
            println!("Would fetch in: {}", path);
        }
        cli::Commands::Pull { path, .. } => {
            println!("Would pull in: {}", path);
        }
        cli::Commands::Status { path } => {
            println!("Would show status for: {}", path);
        }
        cli::Commands::Init { path } => {
            println!("Would create config in: {}", path);
        }
    }
}
```

**Done when:** All these commands work:
```bash
cargo run -- --help
cargo run -- clone --help
cargo run -- clone ~/github --dry-run
cargo run -- fetch ~/github
cargo run -- init
```

---

## Phase 3: Authentication

### Task 3.1: Implement GitHub CLI Token Retrieval

**File:** `src/auth/gh_cli.rs`

```rust
use anyhow::{bail, Context, Result};
use std::process::Command;

/// Check if GitHub CLI is installed
pub fn is_gh_installed() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if user is authenticated with GitHub CLI
pub fn is_gh_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get GitHub token from gh CLI
pub fn get_token() -> Result<String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("Failed to run 'gh auth token'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh auth token failed: {}", stderr);
    }

    let token = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in token")?
        .trim()
        .to_string();

    if token.is_empty() {
        bail!("gh auth token returned empty token");
    }

    Ok(token)
}

/// Get the authenticated GitHub username
pub fn get_username() -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .context("Failed to get username from gh")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to get username: {}", stderr);
    }

    let username = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in username")?
        .trim()
        .to_string();

    Ok(username)
}
```

**File:** `src/auth/mod.rs`

```rust
pub mod gh_cli;

use anyhow::{bail, Result};
use std::env;

/// Authentication token and method used
#[derive(Debug)]
pub struct Auth {
    pub token: String,
    pub method: AuthMethod,
    pub username: String,
}

#[derive(Debug)]
pub enum AuthMethod {
    GhCli,
    EnvVar(String),  // Which env var was used
}

/// Get authentication token, trying methods in priority order
pub fn get_auth() -> Result<Auth> {
    // Priority 1: GitHub CLI
    if gh_cli::is_gh_installed() && gh_cli::is_gh_authenticated() {
        let token = gh_cli::get_token()?;
        let username = gh_cli::get_username()?;
        return Ok(Auth {
            token,
            method: AuthMethod::GhCli,
            username,
        });
    }

    // Priority 2: Environment variables
    for var_name in ["GITHUB_TOKEN", "GH_TOKEN", "GISA_TOKEN"] {
        if let Ok(token) = env::var(var_name) {
            if !token.is_empty() {
                // We need to fetch username via API since we don't have gh
                // For now, return a placeholder - will be filled by discovery
                return Ok(Auth {
                    token,
                    method: AuthMethod::EnvVar(var_name.to_string()),
                    username: String::new(),  // Will be fetched later
                });
            }
        }
    }

    // No auth found
    bail!(
        "No GitHub authentication found.\n\n\
         Please authenticate using one of these methods:\n\n\
         1. GitHub CLI (recommended):\n   \
            gh auth login\n\n\
         2. Environment variable:\n   \
            export GITHUB_TOKEN=ghp_xxxx\n\n\
         For more info: https://cli.github.com/manual/gh_auth_login"
    );
}
```

**Test manually:**
```rust
// Temporary test in main.rs
use mod auth;

fn main() {
    match auth::get_auth() {
        Ok(auth) => println!("Authenticated as: {} via {:?}", auth.username, auth.method),
        Err(e) => eprintln!("Auth failed: {}", e),
    }
}
```

**Done when:** Running `cargo run` shows your GitHub username (if `gh` is installed and authenticated).

---

## Phase 4: GitHub API Discovery

### Task 4.1: Implement GitHub API Client

**File:** `src/discovery/github.rs`

```rust
use crate::types::{Org, Repo, OwnedRepo};
use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT, ACCEPT};
use reqwest::Client;
use serde::de::DeserializeOwned;

const GITHUB_API_URL: &str = "https://api.github.com";

/// GitHub API client
pub struct GitHubClient {
    client: Client,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub API client
    pub fn new(token: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gisa-cli"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, token })
    }

    /// Make an authenticated GET request
    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self.client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await
            .context("HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("GitHub API error ({}): {}", status, body);
        }

        response.json().await.context("Failed to parse JSON response")
    }

    /// Fetch all pages of a paginated endpoint
    async fn get_all_pages<T: DeserializeOwned>(&self, base_url: &str) -> Result<Vec<T>> {
        let mut results = Vec::new();
        let mut page = 1;

        loop {
            let url = format!("{}?per_page=100&page={}", base_url, page);
            let items: Vec<T> = self.get(&url).await?;

            if items.is_empty() {
                break;
            }

            results.extend(items);
            page += 1;

            // Safety limit to prevent infinite loops
            if page > 100 {
                break;
            }
        }

        Ok(results)
    }

    /// Get the authenticated user's login
    pub async fn get_username(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct User {
            login: String,
        }

        let user: User = self.get(&format!("{}/user", GITHUB_API_URL)).await?;
        Ok(user.login)
    }

    /// Fetch all organizations the user belongs to
    pub async fn get_orgs(&self) -> Result<Vec<Org>> {
        self.get_all_pages(&format!("{}/user/orgs", GITHUB_API_URL)).await
    }

    /// Fetch all repos for an organization
    pub async fn get_org_repos(&self, org: &str) -> Result<Vec<Repo>> {
        self.get_all_pages(&format!("{}/orgs/{}/repos", GITHUB_API_URL, org)).await
    }

    /// Fetch user's personal repos (owned by them, not org repos)
    pub async fn get_user_repos(&self) -> Result<Vec<Repo>> {
        self.get_all_pages(&format!("{}/user/repos?affiliation=owner", GITHUB_API_URL)).await
    }
}

/// Discover all repos the user has access to
pub async fn discover_all(token: &str, include_archived: bool, include_forks: bool) -> Result<Vec<OwnedRepo>> {
    let client = GitHubClient::new(token.to_string())?;
    let username = client.get_username().await?;

    let mut all_repos = Vec::new();

    // Fetch orgs and their repos
    let orgs = client.get_orgs().await?;
    for org in &orgs {
        let repos = client.get_org_repos(&org.login).await?;
        for repo in repos {
            // Apply filters
            if !include_archived && repo.archived {
                continue;
            }
            if !include_forks && repo.fork {
                continue;
            }

            all_repos.push(OwnedRepo {
                owner: org.login.clone(),
                repo,
            });
        }
    }

    // Fetch personal repos
    let personal_repos = client.get_user_repos().await?;
    for repo in personal_repos {
        // Skip if already added via org
        if all_repos.iter().any(|r| r.repo.id == repo.id) {
            continue;
        }

        if !include_archived && repo.archived {
            continue;
        }
        if !include_forks && repo.fork {
            continue;
        }

        all_repos.push(OwnedRepo {
            owner: username.clone(),
            repo,
        });
    }

    Ok(all_repos)
}
```

**File:** `src/discovery/mod.rs`
```rust
pub mod github;
pub use github::{discover_all, GitHubClient};
```

**Done when:** You can call `discover_all` and get a list of repos.

---

### Task 4.2: Add Progress Reporting to Discovery

**Update \****`src/discovery/github.rs`** to add progress callbacks:

```rust
use indicatif::{ProgressBar, ProgressStyle};

/// Discover all repos with progress reporting
pub async fn discover_all_with_progress(
    token: &str,
    include_archived: bool,
    include_forks: bool,
) -> Result<Vec<OwnedRepo>> {
    let client = GitHubClient::new(token.to_string())?;

    // Spinner for initial fetch
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );

    spinner.set_message("Fetching GitHub username...");
    let username = client.get_username().await?;
    spinner.set_message(format!("Authenticated as {}", username));

    spinner.set_message("Fetching organizations...");
    let orgs = client.get_orgs().await?;
    spinner.finish_with_message(format!("Found {} organizations", orgs.len()));

    // Progress bar for org repos
    let mut all_repos = Vec::new();

    if !orgs.is_empty() {
        let pb = ProgressBar::new(orgs.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} orgs - {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        for org in &orgs {
            pb.set_message(org.login.clone());
            let repos = client.get_org_repos(&org.login).await?;

            for repo in repos {
                if !include_archived && repo.archived { continue; }
                if !include_forks && repo.fork { continue; }

                all_repos.push(OwnedRepo {
                    owner: org.login.clone(),
                    repo,
                });
            }
            pb.inc(1);
        }
        pb.finish_with_message("Organizations complete");
    }

    // Personal repos
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Fetching personal repositories...");

    let personal_repos = client.get_user_repos().await?;
    let mut personal_count = 0;

    for repo in personal_repos {
        if all_repos.iter().any(|r| r.repo.id == repo.id) { continue; }
        if !include_archived && repo.archived { continue; }
        if !include_forks && repo.fork { continue; }

        all_repos.push(OwnedRepo {
            owner: username.clone(),
            repo,
        });
        personal_count += 1;
    }

    spinner.finish_with_message(format!("Found {} personal repositories", personal_count));

    println!("\n✓ Discovered {} total repositories\n", all_repos.len());

    Ok(all_repos)
}
```

**Done when:** Running discovery shows progress bars.

---

## Phase 5: Clone Manager

### Task 5.1: Implement Git Clone Operations

**File:** `src/clone/parallel.rs`

```rust
use crate::types::{OwnedRepo, OpResult};
use crate::config::Config;
use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Command;
use tokio::sync::Semaphore;
use std::sync::Arc;

/// Clone a single repository
fn clone_repo(repo: &OwnedRepo, target_path: &Path, config: &Config) -> OpResult {
    // Check if already exists
    if target_path.exists() {
        return OpResult::Skipped("Already exists".to_string());
    }

    // Create parent directory
    if let Some(parent) = target_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return OpResult::Failed(format!("Failed to create directory: {}", e));
        }
    }

    // Build git clone command
    let mut cmd = Command::new("git");
    cmd.args(["clone", "--progress"]);

    // Clone options
    if config.clone.depth > 0 {
        cmd.args(["--depth", &config.clone.depth.to_string()]);
    }
    if !config.clone.branch.is_empty() {
        cmd.args(["--branch", &config.clone.branch]);
    }
    if config.clone.recurse_submodules {
        cmd.arg("--recurse-submodules");
    }

    // Use SSH URL
    cmd.arg(&repo.repo.ssh_url);
    cmd.arg(target_path);

    // Run clone
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => return OpResult::Failed(format!("Failed to run git: {}", e)),
    };

    if output.status.success() {
        OpResult::Success
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        OpResult::Failed(stderr.to_string())
    }
}

/// Clone multiple repositories in parallel
pub async fn clone_repos(
    repos: Vec<OwnedRepo>,
    config: &Config,
    dry_run: bool,
) -> Result<Vec<(OwnedRepo, OpResult)>> {
    let mp = MultiProgress::new();
    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    let pb = mp.add(ProgressBar::new(repos.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} - {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut results = Vec::new();

    for repo in repos {
        let target_path = config.repo_path(&repo.owner, &repo.repo.name)?;

        pb.set_message(repo.repo.full_name.clone());

        if dry_run {
            println!("  Would clone: {} -> {}", repo.repo.full_name, target_path.display());
            results.push((repo, OpResult::Skipped("Dry run".to_string())));
        } else {
            // Acquire semaphore permit (limits concurrency)
            let _permit = semaphore.acquire().await?;

            let result = clone_repo(&repo, &target_path, config);
            results.push((repo, result));
        }

        pb.inc(1);
    }

    pb.finish_with_message("Complete");

    Ok(results)
}

/// Print summary of clone results
pub fn print_summary(results: &[(OwnedRepo, OpResult)]) {
    let success = results.iter().filter(|(_, r)| matches!(r, OpResult::Success)).count();
    let skipped = results.iter().filter(|(_, r)| matches!(r, OpResult::Skipped(_))).count();
    let failed: Vec<_> = results.iter()
        .filter_map(|(repo, r)| {
            if let OpResult::Failed(msg) = r {
                Some((repo, msg))
            } else {
                None
            }
        })
        .collect();

    println!("\n=== Summary ===");
    println!("✓ Cloned: {}", success);
    println!("○ Skipped: {}", skipped);
    println!("✗ Failed: {}", failed.len());

    if !failed.is_empty() {
        println!("\nFailed repositories:");
        for (repo, msg) in failed {
            println!("  {} - {}", repo.repo.full_name, msg);
        }
    }
}
```

**File:** `src/clone/mod.rs`
```rust
pub mod parallel;
pub use parallel::{clone_repos, print_summary};
```

**Done when:** `cargo check` passes.

---

## Phase 6: Sync Manager

### Task 6.1: Implement Fetch/Pull Operations

**File:** `src/sync/manager.rs`

```rust
use crate::types::{OwnedRepo, OpResult};
use crate::config::Config;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Command;

/// Check if a directory is a git repository
fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Check if a repo has uncommitted changes
fn has_uncommitted_changes(path: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => true, // Assume dirty if we can't check
    }
}

/// Fetch updates for a repository
fn fetch_repo(path: &Path) -> OpResult {
    let output = Command::new("git")
        .args(["fetch", "--all", "--prune"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => OpResult::Success,
        Ok(o) => OpResult::Failed(String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => OpResult::Failed(e.to_string()),
    }
}

/// Pull updates for a repository
fn pull_repo(path: &Path) -> OpResult {
    // First check for uncommitted changes
    if has_uncommitted_changes(path) {
        return OpResult::Skipped("Has uncommitted changes".to_string());
    }

    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => OpResult::Success,
        Ok(o) => OpResult::Failed(String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => OpResult::Failed(e.to_string()),
    }
}

/// Sync mode
pub enum SyncMode {
    Fetch,
    Pull,
}

/// Find all existing repos under the base path
pub fn find_existing_repos(base_path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut repos = Vec::new();

    // Walk directory structure: base/org/repo
    if let Ok(orgs) = std::fs::read_dir(base_path) {
        for org_entry in orgs.flatten() {
            if !org_entry.path().is_dir() { continue; }

            if let Ok(repos_in_org) = std::fs::read_dir(org_entry.path()) {
                for repo_entry in repos_in_org.flatten() {
                    let repo_path = repo_entry.path();
                    if repo_path.is_dir() && is_git_repo(&repo_path) {
                        repos.push(repo_path);
                    }
                }
            }
        }
    }

    Ok(repos)
}

/// Sync all repos in the base path
pub fn sync_repos(
    base_path: &Path,
    mode: SyncMode,
    dry_run: bool,
) -> Result<Vec<(std::path::PathBuf, OpResult)>> {
    let repos = find_existing_repos(base_path)?;

    let pb = ProgressBar::new(repos.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} - {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut results = Vec::new();

    let mode_name = match mode {
        SyncMode::Fetch => "fetch",
        SyncMode::Pull => "pull",
    };

    for repo_path in repos {
        let name = repo_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        pb.set_message(name);

        let result = if dry_run {
            println!("  Would {}: {}", mode_name, repo_path.display());
            OpResult::Skipped("Dry run".to_string())
        } else {
            match mode {
                SyncMode::Fetch => fetch_repo(&repo_path),
                SyncMode::Pull => pull_repo(&repo_path),
            }
        };

        results.push((repo_path, result));
        pb.inc(1);
    }

    pb.finish_with_message("Complete");

    Ok(results)
}

/// Print sync summary
pub fn print_sync_summary(results: &[(std::path::PathBuf, OpResult)]) {
    let success = results.iter().filter(|(_, r)| matches!(r, OpResult::Success)).count();
    let skipped = results.iter().filter(|(_, r)| matches!(r, OpResult::Skipped(_))).count();
    let failed: Vec<_> = results.iter()
        .filter_map(|(path, r)| {
            if let OpResult::Failed(msg) = r {
                Some((path, msg))
            } else {
                None
            }
        })
        .collect();

    println!("\n=== Sync Summary ===");
    println!("✓ Synced: {}", success);
    println!("○ Skipped: {}", skipped);
    println!("✗ Failed: {}", failed.len());

    if !failed.is_empty() {
        println!("\nFailed repositories:");
        for (path, msg) in failed {
            println!("  {} - {}", path.display(), msg);
        }
    }
}
```

**File:** `src/sync/mod.rs`
```rust
pub mod manager;
pub use manager::{sync_repos, print_sync_summary, SyncMode, find_existing_repos};
```

**Done when:** `cargo check` passes.

---

## Phase 7: Wire Everything Together

### Task 7.1: Complete Main.rs

**File:** `src/main.rs`

```rust
mod cli;
mod config;
mod auth;
mod discovery;
mod clone;
mod sync;
mod types;

use anyhow::Result;
use config::Config;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::parse();

    // Load config file if specified or from default location
    let config_path = args.config.unwrap_or_else(|| Path::new("gisa.config.toml").to_path_buf());
    let mut config = Config::load(&config_path)?;

    match args.command {
        cli::Commands::Clone {
            path,
            jobs,
            dry_run,
            depth,
            recurse_submodules,
            include_archived,
            include_forks,
            refresh: _,
        } => {
            // Override config with CLI args
            config.base_path = path;
            if let Some(j) = jobs {
                config.concurrency = j;
            }
            if let Some(d) = depth {
                config.clone.depth = d;
            }
            if recurse_submodules {
                config.clone.recurse_submodules = true;
            }
            if include_archived {
                config.filters.include_archived = true;
            }
            if include_forks {
                config.filters.include_forks = true;
            }

            // Authenticate
            println!("Authenticating...\n");
            let auth = auth::get_auth()?;
            println!("✓ Authenticated as {} via {:?}\n", auth.username, auth.method);

            // Discover repos
            println!("Discovering repositories...\n");
            let repos = discovery::discover_all_with_progress(
                &auth.token,
                config.filters.include_archived,
                config.filters.include_forks,
            ).await?;

            if repos.is_empty() {
                println!("No repositories found.");
                return Ok(());
            }

            // Clone
            if dry_run {
                println!("=== Dry Run ===\n");
            }
            println!("Cloning {} repositories...\n", repos.len());

            let results = clone::clone_repos(repos, &config, dry_run).await?;
            clone::print_summary(&results);
        }

        cli::Commands::Fetch { path, jobs, dry_run } => {
            config.base_path = path;
            if let Some(j) = jobs {
                config.concurrency = j;
            }

            let base_path = config.expanded_base_path()?;
            println!("Fetching repos in {}...\n", base_path.display());

            let results = sync::sync_repos(&base_path, sync::SyncMode::Fetch, dry_run)?;
            sync::print_sync_summary(&results);
        }

        cli::Commands::Pull { path, jobs, dry_run } => {
            config.base_path = path;
            if let Some(j) = jobs {
                config.concurrency = j;
            }

            let base_path = config.expanded_base_path()?;
            println!("Pulling repos in {}...\n", base_path.display());

            let results = sync::sync_repos(&base_path, sync::SyncMode::Pull, dry_run)?;
            sync::print_sync_summary(&results);
        }

        cli::Commands::Status { path } => {
            config.base_path = path;
            let base_path = config.expanded_base_path()?;

            let repos = sync::find_existing_repos(&base_path)?;
            println!("Found {} repositories in {}\n", repos.len(), base_path.display());

            for repo in repos {
                let name = repo.strip_prefix(&base_path)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| repo.display().to_string());
                println!("  {}", name);
            }
        }

        cli::Commands::Init { path } => {
            let config_file = Path::new(&path).join("gisa.config.toml");

            if config_file.exists() {
                println!("Config file already exists: {}", config_file.display());
                return Ok(());
            }

            let default_config = r#"# Gisa configuration file
# See: https://github.com/user/gisa for documentation

# Base directory for all cloned repos
base_path = "~/github"

# Directory structure pattern
# {org} = organization name or GitHub username
# {repo} = repository name
structure = "{org}/{repo}"

# Number of parallel clone/sync operations
concurrency = 4

# Sync behavior: "fetch" (safe) or "pull" (updates working tree)
sync_mode = "fetch"

[clone]
# Clone depth (0 = full history)
depth = 0

# Clone submodules
recurse_submodules = false

[filters]
# Include archived repositories
include_archived = false

# Include forked repositories
include_forks = false
"#;

            std::fs::write(&config_file, default_config)?;
            println!("Created: {}", config_file.display());
        }
    }

    Ok(())
}
```

**Done when:** All commands work end-to-end:
```bash
cargo run -- init
cargo run -- clone ~/github --dry-run
cargo run -- status ~/github
```

---

## Phase 8: Testing and Polish

### Task 8.1: Add Integration Tests

**File:** `tests/integration_test.rs`

```rust
use std::process::Command;

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gisa"));
    assert!(stdout.contains("clone"));
    assert!(stdout.contains("fetch"));
}

#[test]
fn test_init_creates_config() {
    let temp_dir = tempfile::tempdir().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "init", temp_dir.path().to_str().unwrap()])
        .output()
        .expect("Failed to run command");

    assert!(output.status.success());
    assert!(temp_dir.path().join("gisa.config.toml").exists());
}

#[test]
fn test_clone_dry_run() {
    let output = Command::new("cargo")
        .args(["run", "--", "clone", "/tmp/test", "--dry-run"])
        .output()
        .expect("Failed to run command");

    // Should fail gracefully if not authenticated
    // (the dry-run flag should be recognized)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("error: unrecognized"));
}
```

**Done when:** `cargo test` passes all tests.

---

### Task 8.2: Add Shell Completions

**Update \****`src/cli.rs`** to add completion generation:

```rust
use clap::CommandFactory;
use clap_complete::{generate, Shell};

/// Generate shell completions to stdout
pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "gisa", &mut std::io::stdout());
}
```

**Add completion command to CLI:**
```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}
```

**Add to Cargo.toml:**
```toml
clap_complete = "4"
```

**Done when:** `cargo run -- completions bash` outputs bash completions.

---

## Phase 9: Build and Release

### Task 9.1: Create Release Build Script

**File:** `scripts/build-release.sh`

```bash
#!/bin/bash
set -e

VERSION=${1:-"0.1.0"}
TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-unknown-linux-gnu"
)

mkdir -p dist

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    cargo build --release --target "$target"

    if [[ "$target" == *"windows"* ]]; then
        cp "target/$target/release/gisa.exe" "dist/gisa-$VERSION-$target.exe"
    else
        cp "target/$target/release/gisa" "dist/gisa-$VERSION-$target"
    fi
done

echo "Release builds complete in dist/"
ls -la dist/
```

**Done when:** `./scripts/build-release.sh` creates release binaries.

---

### Task 9.2: Create Homebrew Formula

**File:** `Formula/gisa.rb` (for your tap)

```ruby
class Gisa < Formula
  desc "Mirror GitHub org/repo structure locally"
  homepage "https://github.com/yourusername/gisa"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yourusername/gisa/releases/download/v0.1.0/gisa-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    else
      url "https://github.com/yourusername/gisa/releases/download/v0.1.0/gisa-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    bin.install "gisa"

    # Install shell completions
    generate_completions_from_executable(bin/"gisa", "completions")
  end

  test do
    assert_match "gisa", shell_output("#{bin}/gisa --version")
  end
end
```

**Done when:** Formula file is ready for publishing.

---

## Verification Checklist

After completing all phases, verify:

- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes all tests
- [ ] `gisa --help` shows all commands
- [ ] `gisa init` creates config file
- [ ] `gisa clone ~/test --dry-run` shows discovered repos
- [ ] `gisa clone ~/test` clones all repos (with auth)
- [ ] `gisa fetch ~/test` fetches updates
- [ ] `gisa pull ~/test` pulls updates
- [ ] `gisa status ~/test` lists repos
- [ ] `gisa completions bash` outputs completions
- [ ] Binary size is reasonable (~5MB)
- [ ] Runs without Rust toolchain installed

---

## Quick Reference: File to Create

| File | Purpose |
| --- | --- |
| `src/main.rs` | Entry point, command dispatch |
| `src/lib.rs` | Module exports |
| `src/cli.rs` | Argument parsing (clap) |
| `src/types.rs` | Shared data structures |
| `src/config/mod.rs` | Config module |
| `src/config/parser.rs` | TOML parsing |
| `src/auth/mod.rs` | Auth module |
| `src/auth/gh_cli.rs` | GitHub CLI integration |
| `src/discovery/mod.rs` | Discovery module |
| `src/discovery/github.rs` | GitHub API client |
| `src/clone/mod.rs` | Clone module |
| `src/clone/parallel.rs` | Parallel cloning |
| `src/sync/mod.rs` | Sync module |
| `src/sync/manager.rs` | Fetch/pull logic |

---

## Dependency Quick Reference

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
clap_complete = "4"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
indicatif = "0.17"
console = "0.15"
directories = "5"
thiserror = "1"
anyhow = "1"
shellexpand = "3"

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
```

---

## Troubleshooting Guide

### Common Errors and Solutions

#### SSH Key Issues

**Error:** `Permission denied (publickey)`

**Cause:** SSH key not configured or not added to GitHub.

**Fix:**
```bash
# Check if SSH key exists
ls -la ~/.ssh/id_ed25519.pub

# If not, create one
ssh-keygen -t ed25519 -C "your_email@example.com"

# Add to SSH agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# Copy public key and add to GitHub Settings > SSH Keys
cat ~/.ssh/id_ed25519.pub

# Test connection
ssh -T git@github.com
```

---

#### GitHub API Rate Limits

**Error:** `GitHub API error (403): rate limit exceeded`

**Cause:** Too many API requests. Unauthenticated: 60/hour. Authenticated: 5,000/hour.

**Fix:**
```bash
# Check current rate limit
gh api rate_limit

# Wait for reset (shown in response) or ensure authentication is working
gh auth status
```

**Prevention:** The code uses authenticated requests which have much higher limits.

---

#### gh CLI Not Authenticated

**Error:** `No GitHub authentication found`

**Fix:**
```bash
# Interactive login (opens browser)
gh auth login

# Or with token
gh auth login --with-token < token.txt

# Verify
gh auth status
```

---

#### Clone Fails for Private Repo

**Error:** `Repository not found` or `Could not read from remote repository`

**Causes:**
1. Token lacks `repo` scope
2. SSH key not added to GitHub
3. Not a member of the organization

**Fix:**
```bash
# Check token scopes
gh auth status

# Re-authenticate with correct scopes
gh auth login --scopes repo,read:org

# For SSH issues, test connection
ssh -T git@github.com
```

---

#### Network/Timeout Errors

**Error:** `HTTP request failed` or `Connection refused`

**Causes:**
1. No internet connection
2. Firewall blocking GitHub
3. GitHub is down

**Fix:**
```bash
# Test basic connectivity
ping github.com
curl -I https://api.github.com

# Check GitHub status
open https://www.githubstatus.com
```

---

#### "Already exists" for All Repos

**Symptom:** Every repo shows "Skipped: Already exists" but directories are empty or wrong.

**Cause:** Target directory exists but isn't a git repo (maybe failed previous clone).

**Fix:**
```bash
# Remove empty/broken directories
find ~/github -type d -empty -delete

# Or manually remove and re-clone
rm -rf ~/github/org/repo
gisa clone ~/github
```

---

#### Pull Fails with "Uncommitted Changes"

**Error:** `Skipped: Has uncommitted changes`

**Cause:** Local modifications exist that would be overwritten.

**Fix:**
```bash
# Go to the repo
cd ~/github/org/repo

# Check what changed
git status

# Either commit, stash, or discard changes
git stash        # Save for later
git checkout .   # Discard all changes
```

---

#### Rust Compilation Errors

**Error:** `error[E0433]: failed to resolve: use of undeclared crate or module`

**Cause:** Missing module declaration or import.

**Fix checklist:**
1. Did you add `pub mod modulename;` to `mod.rs` or `lib.rs`?
2. Did you add `use crate::modulename;` where needed?
3. Did you run `cargo check` after creating new files?

---

## Code Review Checklist

Before marking any task complete, verify:

### Functionality
- [ ] Code compiles: `cargo check`
- [ ] Tests pass: `cargo test`
- [ ] Feature works manually (run and test)

### Code Quality
- [ ] No `unwrap()` on user input or external data (use `?` or proper error handling)
- [ ] No hardcoded paths (use config or CLI args)
- [ ] No secrets in code (tokens come from env/gh CLI)
- [ ] Error messages are helpful (tell user what went wrong AND how to fix it)

### Rust Specifics
- [ ] No compiler warnings: `cargo check 2>&1 | grep warning`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Code is formatted: `cargo fmt`

### Documentation
- [ ] Public functions have `///` doc comments
- [ ] Complex logic has inline comments explaining WHY
- [ ] README updated if user-facing behavior changed

---

## Git Commit Guide

### When to Commit

Commit after completing each **task** (not each phase). Small, focused commits are easier to review and revert.

### Commit Message Format

```
<Verb>: <what was done>

<optional body explaining WHY, not WHAT>
```

**Format rules:**
- Start with a present-tense verb (Add, Create, Implement, Fix, Update, Remove)
- Describe what was done, not what will be done
- Keep under 50 characters

**Examples:**
- `Add config parser with TOML support`
- `Create module structure for auth and discovery`
- `Implement parallel clone manager`
- `Fix rate limit handling in GitHub client`
- `Update error messages with troubleshooting hints`
- `Remove deprecated sync option`

### Commit Schedule

| After Task | Commit Message |
| --- | --- |
| 0.1 | `Create Rust project skeleton` |
| 0.2 | `Add project dependencies` |
| 0.3 | `Create module structure` |
| 1.1 | `Add core type definitions` |
| 1.2 | `Implement config parser` |
| 1.3 | `Add config parser tests` |
| 2.1 | `Implement CLI argument parsing` |
| 3.1 | `Add GitHub CLI authentication` |
| 4.1 | `Implement GitHub API client` |
| 4.2 | `Add progress reporting to discovery` |
| 5.1 | `Implement parallel clone manager` |
| 6.1 | `Implement fetch and pull sync operations` |
| 7.1 | `Wire up main.rs with all commands` |
| 8.1 | `Add integration tests` |
| 8.2 | `Add shell completion generation` |
| 9.1 | `Add release build script` |
| 9.2 | `Add Homebrew formula` |

### Example Workflow

```bash
# After completing Task 1.2
git add src/config/
git commit -m "Implement config parser

Support TOML config files with defaults for all options.
Handle ~ expansion in paths."

# After completing Task 3.1
git add src/auth/
git commit -m "Add GitHub CLI authentication

Try gh CLI first, fall back to GITHUB_TOKEN env var.
Provide helpful error message if no auth found."
```

---

## "I'm Stuck" Decision Tree

```
commit messagSTART: Something isn't working
  │
  ├─► Does it compile? (`cargo check`)
  │     │
  │     NO ──► Read the error message carefully
  │     │       │
  │     │       ├─► "cannot find" ──► Missing `use` or `mod` statement
  │     │       ├─► "expected X, found Y" ──► Type mismatch, check function signatures
  │     │       └─► "borrowed value" ──► Ownership issue, try `.clone()` or `&`
  │     │
  │     YES ──► Continue below
  │
  ├─► Do tests pass? (`cargo test`)
  │     │
  │     NO ──► Read which test failed and why
  │     │       │
  │     │       └─► Compare expected vs actual output
  │     │
  │     YES ──► Continue below
  │
  ├─► Does it run? (`cargo run -- <command>`)
  │     │
  │     NO ──► Check the runtime error
  │     │       │
  │     │       ├─► "No GitHub authentication" ──► Run `gh auth login`
  │     │       ├─► "Permission denied" ──► SSH key issue (see above)
  │     │       └─► "rate limit" ──► Wait or check auth
  │     │
  │     YES ──► Continue below
  │
  └─► Does it do the right thing?
        │
        NO ──► Add debug prints
        │       │
        │       └─► `println!("DEBUG: {:?}", variable);`
        │           Run again and trace the values
        │
        YES ──► Task complete! Commit and move on.
```

---

## Quick Commands Reference

```bash
# Development
cargo check          # Fast compile check (no binary)
cargo build          # Build debug binary
cargo run -- clone   # Run with arguments
cargo test           # Run all tests
cargo test config    # Run tests matching "config"
cargo fmt            # Format code
cargo clippy         # Lint code

# Git
git status           # See what changed
git diff             # See changes in detail
git add -p           # Stage changes interactively
git commit -m "msg"  # Commit with message
git log --oneline -5 # See recent commits

# GitHub CLI
gh auth status       # Check authentication
gh auth login        # Authenticate
gh api rate_limit    # Check API rate limits
gh api user          # Test API access

# Debugging
RUST_BACKTRACE=1 cargo run -- clone  # Show stack trace on panic
cargo run -- clone --dry-run         # Preview without doing
```

---

## First-Run Experience

When a user runs `gisa` for the first time, provide a guided setup:

### Task: Implement First-Run Detection

**File:** `src/main.rs` (add before command dispatch)

```rust
use directories::ProjectDirs;

fn is_first_run() -> bool {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "gisa") {
        let config_dir = proj_dirs.config_dir();
        !config_dir.join(".initialized").exists()
    } else {
        true
    }
}

fn mark_initialized() -> Result<()> {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "gisa") {
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        std::fs::write(config_dir.join(".initialized"), "")?;
    }
    Ok(())
}

fn run_first_time_setup() -> Result<()> {
    println!("Welcome to Gisa!\n");
    println!("Gisa mirrors your GitHub organizations and repositories locally.\n");

    // Check prerequisites
    println!("Checking prerequisites...\n");

    // 1. Check git
    print!("  Git installed: ");
    if Command::new("git").arg("--version").output().is_ok() {
        println!("✓");
    } else {
        println!("✗");
        println!("\n  Please install git: https://git-scm.com/downloads");
        std::process::exit(1);
    }

    // 2. Check gh CLI
    print!("  GitHub CLI installed: ");
    if Command::new("gh").arg("--version").output().is_ok() {
        println!("✓");
    } else {
        println!("✗ (optional, but recommended)");
        println!("    Install: https://cli.github.com");
    }

    // 3. Check authentication
    print!("  GitHub authenticated: ");
    match auth::get_auth() {
        Ok(auth) => println!("✓ ({})", auth.username),
        Err(_) => {
            println!("✗");
            println!("\n  Please authenticate:");
            println!("    gh auth login");
            println!("  Or set environment variable:");
            println!("    export GITHUB_TOKEN=ghp_xxxx");
            std::process::exit(1);
        }
    }

    println!("\nSetup complete! Run 'gisa clone ~/github' to get started.\n");

    mark_initialized()?;
    Ok(())
}
```

**Add to main():**
```rust
// At the start of main(), before command dispatch
if is_first_run() {
    run_first_time_setup()?;
    return Ok(());
}
```

**Done when:** First run shows welcome message and checks prerequisites.

---

## Error Handling Scenarios

| Scenario | Detection | User Message | Recovery |
| --- | --- | --- | --- |
| Git not installed | `git --version` fails | "Git is required. Install: https://git-scm.com" | Exit with code 1 |
| gh CLI not installed | `gh --version` fails | Continue (it's optional) | Fall back to env var |
| gh CLI not authenticated | `gh auth status` fails | "Run 'gh auth login' or set GITHUB_TOKEN" | Exit with code 1 |
| Invalid token | API returns 401 | "Authentication failed. Token may be expired." | Exit with code 1 |
| Rate limit exceeded | API returns 403 + rate limit header | "Rate limit exceeded. Resets at {time}." | Exit with code 1, show reset time |
| Network unreachable | Connection timeout | "Cannot reach GitHub. Check your internet connection." | Exit with code 1 |
| SSH key missing | Clone fails with "Permission denied" | "SSH key not configured. See: {link}" | Skip repo, continue others |
| Disk full | Write fails with ENOSPC | "Disk full. Free up space and retry." | Exit with code 1 |
| Permission denied | Write fails with EACCES | "Cannot write to {path}. Check permissions." | Exit with code 1 |
| Repo already exists | Directory exists | Skip silently (or with --verbose) | Continue to next repo |
| Clone timeout | No progress for 5 minutes | "Clone timed out for {repo}. Skipping." | Skip repo, continue others |

### Implement Error Types

**File:** `src/errors.rs`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GisaError {
    #[error("Git is not installed. Install from: https://git-scm.com")]
    GitNotInstalled,

    #[error("GitHub authentication failed. Run 'gh auth login' or set GITHUB_TOKEN")]
    NotAuthenticated,

    #[error("Authentication token is invalid or expired")]
    InvalidToken,

    #[error("GitHub API rate limit exceeded. Resets at {reset_time}")]
    RateLimitExceeded { reset_time: String },

    #[error("Cannot reach GitHub. Check your internet connection")]
    NetworkUnreachable,

    #[error("SSH key not configured for GitHub. See: https://docs.github.com/en/authentication/connecting-to-github-with-ssh")]
    SshKeyMissing,

    #[error("Disk is full. Free up space in {path}")]
    DiskFull { path: String },

    #[error("Permission denied writing to {path}")]
    PermissionDenied { path: String },

    #[error("Clone timed out for {repo}")]
    CloneTimeout { repo: String },

    #[error("{0}")]
    Other(String),
}
```

---

## Logging Strategy

### Log Files Location

```
~/.local/share/gisa/          # Linux
~/Library/Application Support/gisa/   # macOS

├── gisa.log                  # Main application log
├── clone.log                 # Clone operation details
└── sync.log                  # Sync operation details
```

### Log Levels

| Level | When to Use | Example |
| --- | --- | --- |
| ERROR | Operation failed | "Failed to clone repo: permission denied" |
| WARN | Something unexpected but recoverable | "Repo skipped: has uncommitted changes" |
| INFO | Normal operation milestones | "Discovered 47 repositories" |
| DEBUG | Detailed operation info | "GET https://api.github.com/user/orgs" |
| TRACE | Very verbose (API responses, etc.) | Full JSON response bodies |

### Implementation

**Add to Cargo.toml:**
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

**File:** `src/logging.rs`

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use directories::ProjectDirs;

pub fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("gisa=debug")
    } else {
        EnvFilter::new("gisa=info")
    };

    // Console output
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    // File output (if we can get a log directory)
    if let Some(proj_dirs) = ProjectDirs::from("", "", "gisa") {
        let log_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(log_dir).ok();

        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            log_dir,
            "gisa.log",
        );
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .init();
    }
}
```

**Usage:**
```rust
use tracing::{info, warn, error, debug};

info!("Discovered {} repositories", count);
warn!(repo = %repo.name, "Skipped: has uncommitted changes");
error!(error = %e, "Clone failed");
debug!(url = %url, "Fetching API endpoint");
```

---

## Offline Behavior

### Detection

```rust
fn is_online() -> bool {
    // Try to reach GitHub API
    reqwest::blocking::Client::new()
        .get("https://api.github.com")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .is_ok()
}
```

### Behavior by Command

| Command | Offline Behavior |
| --- | --- |
| `clone` | Error: "Cannot reach GitHub. Check your connection." |
| `fetch` | Error: "Cannot reach GitHub. Check your connection." |
| `pull` | Error: "Cannot reach GitHub. Check your connection." |
| `status` | Works fully offline (reads local filesystem) |
| `init` | Works fully offline (creates local file) |

### Cached Data (Future Enhancement)

Store last-known repo list for offline reference:

```
~/.cache/gisa/
└── repos.json    # Cached repo list from last successful discovery
```

---

## Performance Requirements

| Metric | Target | How to Verify |
| --- | --- | --- |
| Startup time | < 100ms | `time gisa --help` |
| Memory usage (idle) | < 20MB | `ps aux | grep gisa` |
| Memory usage (cloning 100 repos) | < 100MB | Monitor during operation |
| Discovery (100 repos) | < 30 seconds | `time gisa clone --dry-run` |
| Clone throughput | Limited by network | Parallel clones (default: 4) |

### Optimization Checklist

- [ ] Use `--release` builds for production
- [ ] Stream large responses instead of loading into memory
- [ ] Limit concurrent operations to avoid overwhelming network
- [ ] Use shallow clones (--depth 1) for faster initial sync

---

## Version Compatibility

### Minimum Supported Versions

| Dependency | Minimum Version | Check Command |
| --- | --- | --- |
| Git | 2.0.0 | `git --version` |
| GitHub CLI | 2.0.0 | `gh --version` |
| Rust (build only) | 1.70.0 | `rustc --version` |

### Version Check Implementation

```rust
fn check_git_version() -> Result<()> {
    let output = Command::new("git")
        .args(["--version"])
        .output()?;

    let version_str = String::from_utf8_lossy(&output.stdout);
    // Parse "git version 2.39.0" -> "2.39.0"
    let version = version_str
        .split_whitespace()
        .nth(2)
        .ok_or_else(|| anyhow!("Cannot parse git version"))?;

    let parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if parts.get(0).unwrap_or(&0) < &2 {
        bail!("Git 2.0.0 or higher required. Found: {}", version);
    }

    Ok(())
}
```

---

## Data Migration

### Export Configuration

```bash
# Export current config
gisa config export > gisa-backup.toml

# On new machine
gisa config import < gisa-backup.toml
```

### Manual Migration Steps

1. Copy config file:
```bash
   # From old machine
   scp ~/.config/gisa/config.toml newmachine:~/.config/gisa/
```

2. Re-authenticate on new machine:
```bash
   gh auth login
```

3. Re-clone repositories:
```bash
   gisa clone ~/github
```

Note: Git history is not migrated. Repos are cloned fresh from GitHub.

---

## Manual Test Checklist

Before each release, manually verify:

### Setup & Auth
- [ ] First run shows welcome message
- [ ] `gh auth login` workflow works
- [ ] `GITHUB_TOKEN` env var works
- [ ] Clear error when no auth configured

### Clone Command
- [ ] `gisa clone ~/test --dry-run` shows repos without cloning
- [ ] `gisa clone ~/test` clones all accessible repos
- [ ] Progress bar shows during clone
- [ ] Skips already-cloned repos
- [ ] Creates org/repo directory structure
- [ ] `--depth 1` creates shallow clones
- [ ] `--include-archived` includes archived repos
- [ ] `--include-forks` includes forked repos
- [ ] `-j 8` runs 8 parallel clones

### Sync Commands
- [ ] `gisa fetch ~/test` fetches all repos
- [ ] `gisa pull ~/test` pulls all repos
- [ ] Pull skips repos with uncommitted changes
- [ ] Summary shows success/skipped/failed counts

### Status Command
- [ ] `gisa status ~/test` lists all repos
- [ ] Shows repo count

### Init Command
- [ ] `gisa init` creates config file
- [ ] Doesn't overwrite existing config
- [ ] Created config is valid TOML

### Error Handling
- [ ] Graceful error when offline
- [ ] Graceful error when rate limited
- [ ] Graceful error when SSH key missing
- [ ] Continues after single repo failure

### Edge Cases
- [ ] Works with 0 repositories
- [ ] Works with 500+ repositories
- [ ] Handles repos with special characters in names
- [ ] Handles very long organization names

---

## Security Considerations

### Token Storage

**Never store tokens in:**
- Source code
- Git history
- Plain text config files
- Environment variables in scripts checked into git

**Safe token sources (in order of preference):**
1. GitHub CLI (`gh auth token`) - tokens stored in system keychain
2. Environment variable set in shell profile (not in repo)
3. OS keychain/credential manager

### HTTPS Enforcement

All GitHub API calls must use HTTPS. The code already enforces this:

```rust
const GITHUB_API_URL: &str = "https://api.github.com";  // Always HTTPS
```

For git clone operations, prefer SSH URLs over HTTPS to avoid credential prompts:
```rust
cmd.arg(&repo.repo.ssh_url);  // git@github.com:org/repo.git
```

### Dependency Auditing

Run security audits regularly:

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Fix vulnerabilities
cargo update  # Update to patched versions
```

**Add to CI pipeline** (see CI/CD section below).

### Input Validation

Sanitize all user input that becomes file paths or shell commands:

```rust
fn sanitize_path_component(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect()
}

// Usage: prevent directory traversal
let safe_org = sanitize_path_component(&org.login);
let safe_repo = sanitize_path_component(&repo.name);
let path = base_path.join(safe_org).join(safe_repo);
```

### Secrets in Logs

Never log tokens or sensitive data:

```rust
// BAD
debug!("Using token: {}", token);

// GOOD
debug!("Using token: {}...", &token[..8]);  // Only first 8 chars
// Or better: don't log tokens at all
debug!("Authentication successful");
```

### Security Checklist

- [ ] Tokens never written to disk by gisa
- [ ] Tokens never logged (even at debug level)
- [ ] All API calls use HTTPS
- [ ] User input sanitized before use in paths
- [ ] Dependencies audited with `cargo audit`
- [ ] No shell injection possible (use Command builder, not shell strings)

---

## Accessibility

### Color-Blind Friendly Output

Don't rely solely on color to convey information. Use symbols too:

```rust
// BAD: Only color distinguishes success/failure
println!("\x1b[32mCloned\x1b[0m");   // Green
println!("\x1b[31mFailed\x1b[0m");   // Red

// GOOD: Symbol + color
println!("✓ Cloned");   // Checkmark for success
println!("✗ Failed");   // X for failure
println!("○ Skipped");  // Circle for skipped
println!("⚠ Warning");  // Warning triangle
```

### Respect NO_COLOR

Honor the `NO_COLOR` environment variable (https://no-color.org):

```rust
fn should_use_color() -> bool {
    // Respect NO_COLOR standard
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    // Also check if stdout is a terminal
    atty::is(atty::Stream::Stdout)
}
```

**Add to Cargo.toml:**
```toml
atty = "0.2"
```

### Progress Bar Accessibility

Indicatif already handles terminal detection, but ensure fallback works:

```rust
let pb = if atty::is(atty::Stream::Stdout) {
    ProgressBar::new(total)
} else {
    // No terminal: use simple line output instead
    ProgressBar::hidden()
};
```

### Screen Reader Friendly Output

- Use clear, complete sentences for important messages
- Avoid ASCII art that doesn't make sense when read aloud
- Put the most important information first

```rust
// BAD: Relies on visual layout
println!("my-org/my-repo .............. ✓");

// GOOD: Clear sentence
println!("✓ Cloned my-org/my-repo");
```

### High Contrast Mode

When using colors, ensure sufficient contrast. Use bold for emphasis:

```rust
use console::Style;

let success = Style::new().green().bold();
let error = Style::new().red().bold();
let warning = Style::new().yellow().bold();

println!("{}", success.apply_to("✓ Clone complete"));
```

### Accessibility Checklist

- [ ] All status indicators use symbols, not just colors
- [ ] `NO_COLOR` environment variable is respected
- [ ] Output works without a terminal (piped to file)
- [ ] Important messages are complete sentences
- [ ] No critical info conveyed only through visual layout

---

## CI/CD Pipeline

### GitHub Actions Workflow

**File:** `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Build
        run: cargo build --verbose

      - name: Run tests
        run: cargo test --verbose

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Security audit
        run: cargo audit

  build-binaries:
    needs: [test, security]
    if: github.ref == 'refs/heads/main'
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: gisa-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/gisa
```

### Release Workflow

**File:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  create-release:
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - name: Create Release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref_name }}
          release_name: ${{ github.ref_name }}
          draft: false
          prerelease: false

  build-and-upload:
    needs: create-release
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: x86_64-apple-darwin
            asset_name: gisa-x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
            asset_name: gisa-aarch64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            asset_name: gisa-x86_64-unknown-linux-gnu

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build release binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Create tarball
        run: |
          cd target/${{ matrix.target }}/release
          tar -czvf ${{ matrix.asset_name }}.tar.gz gisa

      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: target/${{ matrix.target }}/release/${{ matrix.asset_name }}.tar.gz
          asset_name: ${{ matrix.asset_name }}.tar.gz
          asset_content_type: application/gzip
```

### Creating a Release

```bash
# 1. Update version in Cargo.toml
# 2. Commit the change
git add Cargo.toml
git commit -m "Bump version to 0.2.0"

# 3. Create and push tag
git tag v0.2.0
git push origin v0.2.0

# 4. GitHub Actions will automatically:
#    - Run tests
#    - Build binaries for all platforms
#    - Create GitHub release with binaries attached
```

### CI/CD Checklist

- [ ] `.github/workflows/ci.yml` created
- [ ] `.github/workflows/release.yml` created
- [ ] CI runs on every push and PR
- [ ] Tests must pass before merge
- [ ] Security audit runs in CI
- [ ] Release creates binaries for macOS (Intel + ARM) and Linux
- [ ] Binaries are attached to GitHub releases
