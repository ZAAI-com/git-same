# Configuration Specification

## Config File

**Filename**: `config.toml`
**Location**: `~/.config/git-same/config.toml`
**Format**: TOML

## Full Configuration Example

```toml
# ~/.config/git-same/config.toml

# Base directory for all cloned repos
base_path = "~/github"

# Directory structure pattern
# {org} = organization name or GitHub username for personal repos
# {repo} = repository name
# {provider} = provider name (e.g., github)
structure = "{org}/{repo}"

# Number of parallel clone/sync operations
concurrency = 4

# Sync behavior: "fetch" (safe) or "pull" (updates working tree)
sync_mode = "fetch"

[clone]
# Clone depth (0 = full history)
depth = 0

# Clone specific branch (empty = default branch)
branch = ""

# Include submodules
recurse_submodules = false

[filters]
# Include archived repositories
include_archived = false

# Include forked repositories
include_forks = false

# Filter by specific orgs (empty = all)
orgs = []

# Exclude specific repos
exclude_repos = []
```

## Configuration Options

### Core Settings

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `base_path` | string | `"~/github"` | Root directory for cloned repos |
| `structure` | string | `"{org}/{repo}"` | Directory structure pattern |
| `concurrency` | integer | `4` | Parallel operations (1-32) |
| `sync_mode` | string | `"fetch"` | `"fetch"` or `"pull"` |

### Clone Options (`[clone]`)

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `depth` | integer | `0` | Shallow clone depth (0 = full) |
| `branch` | string | `""` | Specific branch (empty = default) |
| `recurse_submodules` | boolean | `false` | Clone submodules |

### Filter Options (`[filters]`)

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `include_archived` | boolean | `false` | Clone archived repos |
| `include_forks` | boolean | `false` | Clone forked repos |
| `orgs` | string[] | `[]` | Filter to specific organizations |
| `exclude_repos` | string[] | `[]` | Exclude specific repos by full name |

### Provider Options (`[[providers]]`)

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `kind` | string | required | `"github"`, `"github-enterprise"` |
| `name` | string | `""` | Display name for this provider |
| `api_url` | string | `""` | API URL (required for GitHub Enterprise) |
| `auth` | string | `"gh-cli"` | `"gh-cli"`, `"env"`, `"token"` |
| `token_env` | string | `""` | Env var name (required when `auth = "env"`) |
| `token` | string | `""` | Token value (required when `auth = "token"`) |
| `prefer_ssh` | boolean | `true` | Use SSH URLs for cloning |
| `base_path` | string | `""` | Override base path for this provider |
| `enabled` | boolean | `true` | Whether this provider is active |

## CLI Flag Overrides

All config options can be overridden via CLI flags:

```bash
# Override concurrency
git-same clone ~/github --concurrency 8

# Override filters
git-same clone ~/github --include-archived --include-forks

# Shallow clone
git-same clone ~/github --depth 1

# Include submodules
git-same clone ~/github --recurse-submodules
```

**Precedence**: CLI flags > config file > defaults

## Minimal Config

For most users, a minimal config is sufficient:

```toml
# ~/.config/git-same/config.toml
base_path = "~/github"
```

All other options use sensible defaults.

## Config Initialization

```bash
# Create default config file
git-same init

# Creates ~/.config/git-same/config.toml with documented defaults
```

## Directory Structure Examples

### Default: `{org}/{repo}`
```
~/github/
├── acme-corp/
│   ├── api/
│   └── web/
└── octocat/           # Personal repos under username
    └── dotfiles/
```

### Flat: `{org}-{repo}`
```
~/github/
├── acme-corp-api/
├── acme-corp-web/
└── octocat-dotfiles/
```

## Defaults Summary

| Setting | Default | Rationale |
| --- | --- | --- |
| `base_path` | `~/github` | Common convention |
| `structure` | `{org}/{repo}` | Mirrors GitHub structure |
| `concurrency` | `4` | Balance speed and system load |
| `sync_mode` | `fetch` | Safe, doesn't modify working tree |
| `depth` | `0` | Full history by default |
| `recurse_submodules` | `false` | Submodules can be large/slow |
| `include_archived` | `false` | Archived = inactive, usually skip |
| `include_forks` | `false` | Forks clutter, clone explicitly if needed |
