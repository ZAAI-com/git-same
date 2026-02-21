# Gisa Architecture Overview

## Quick Start

```bash
# Build
cargo build --release

# Run tests
cargo test

# Try it out
cargo run -- clone ~/test --dry-run
```

**Prerequisites:** Rust toolchain installed, GitHub CLI authenticated (`gh auth login`)

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
- Loads `config.toml` from `~/.config/git-same/`
- TOML configuration format (Rust ecosystem standard)
- Validates and merges CLI flags with config file
- Stores: base path, clone options, concurrency, sync behavior, filters

### 3. Auth Manager
- **Primary**: GitHub CLI (`gh auth token`) integration
- **Fallback 1**: Environment variables (`GITHUB_TOKEN`, `GH_TOKEN`, `GISA_TOKEN`)
- **Fallback 2**: Personal Access Token from config file
- SSH is used for clone operations only, not API authentication
- Token validation before operations begin

### 4. Discovery Module
- Fetches all orgs user belongs to via GitHub API
- Fetches all repos per org (handles pagination)
- Fetches user's personal repos
- Returns unified list with metadata (visibility, clone URLs, archived status)

### 5. Clone Manager
- Parallel cloning with configurable concurrency (default: 4, max: 32)
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
1. User runs: git-same fetch ~/github

2. Auth Manager
   └─→ Obtains GitHub token (gh CLI → env vars → config token)

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

### Configurable via `config.toml`
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
| 1 | Homebrew | `brew install git-same` | macOS users (primary) |
| 2 | GitHub Releases | Download binary | All platforms, no toolchain needed |
| 3 | Cargo | `cargo install git-same` | Rust developers |

### Homebrew (Primary)

```bash
brew install git-same
```

Homebrew formula maintained in homebrew-core or custom tap.

### GitHub Releases

Pre-built binaries for each release:
- `git-same-x86_64-apple-darwin` (macOS Intel)
- `git-same-aarch64-apple-darwin` (macOS Apple Silicon)
- `git-same-x86_64-unknown-linux-gnu` (Linux)
- `git-same-x86_64-pc-windows-msvc.exe` (Windows)

### Cargo (Rust developers)

```bash
cargo install git-same
```

Builds from source via crates.io. Requires Rust toolchain.

## CLI Command Naming

Commands follow standard git naming conventions for familiarity:

| Gisa Command | Git Equivalent | Description |
| --- | --- | --- |
| `git-same clone` | `git clone` | Clone all repos |
| `git-same fetch` | `git fetch` | Fetch updates (safe, no working tree changes) |
| `git-same pull` | `git pull` | Pull updates (modifies working tree) |
| `git-same status` | `git status` | Show sync status of all repos |
| `git-same init` | `git init` | Initialize config file |

## Code Organization

Tests are inline within each module using `#[cfg(test)] mod tests` blocks. Integration tests live in `tests/integration_test.rs`.

```
src/
├── main.rs                  # Entry point, command routing
├── cli.rs                   # Clap CLI definition
├── lib.rs                   # Library root, prelude
├── auth/                    # Multi-strategy authentication
│   ├── mod.rs
│   ├── gh_cli.rs
│   ├── env_token.rs
│   └── ssh.rs
├── cache/                   # TTL-based discovery cache
│   └── mod.rs
├── clone/                   # Parallel clone operations
│   └── parallel.rs
├── completions/             # Shell completion generation
│   └── mod.rs
├── config/                  # TOML config parsing
│   ├── parser.rs
│   └── provider_config.rs
├── discovery/               # Repo discovery & action planning
│   └── mod.rs
├── errors/                  # Error hierarchy (app, git, provider)
│   ├── app.rs
│   ├── git.rs
│   └── provider.rs
├── git/                     # Git operations trait & shell impl
│   ├── traits.rs
│   └── shell.rs
├── output/                  # Progress bars & verbosity
│   └── progress.rs
├── provider/                # Provider trait & implementations
│   ├── traits.rs
│   ├── github/
│   │   ├── client.rs
│   │   └── pagination.rs
│   └── mock.rs
├── sync/                    # Concurrent fetch/pull
│   └── manager.rs
└── types/                   # Core data types
    ├── repo.rs
    └── provider.rs
```

## State Management

### File-Based Cache

No database required. State is managed via simple files:

```
~/.config/git-same/
├── config.toml              # User config
└── cache.json               # Discovery cache (auto-generated)
```

**Cache file** (`cache.json`):
```json
{
  "version": 1,
  "last_discovery": 1705312200,
  "username": "octocat",
  "orgs": ["my-org", "another-org"],
  "repo_count": 45,
  "repos": {
    "github": [
      {
        "owner": "my-org",
        "repo": {
          "full_name": "my-org/repo-one",
          "ssh_url": "git@github.com:my-org/repo-one.git"
        }
      }
    ]
  }
}
```

**Cache behavior:**
- TTL: 1 hour (default, `DEFAULT_CACHE_TTL = 3600`)
- Force refresh with `--refresh` flag
- Skip cache entirely with `--no-cache` flag
- Used to detect new repos without full API scan

## Future Extensibility

The architecture uses a trait-based `Provider` abstraction to support multiple git hosting services:

- **Implemented:** GitHub, GitHub Enterprise
- **Planned:** GitLab, Bitbucket

```
┌─────────────────────────────────────────┐
│           Provider Trait                │
├─────────────────────────────────────────┤
│  + discover_repos(options, progress)    │
│  + rate_limit_info()                    │
│  + get_username()                       │
└─────────────────────────────────────────┘
         ▲           ▲           ▲
         │           │           │
    ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
    │ GitHub  │ │ GitLab  │ │Bitbucket│
    │Provider │ │Provider │ │Provider │
    │   ✅    │ │ planned │ │ planned │
    └─────────┘ └─────────┘ └─────────┘
```
