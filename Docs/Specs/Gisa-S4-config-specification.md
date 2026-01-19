# Configuration Specification

## Config File

**Filename**: `gisa.config.toml`
**Location**: Project directory (where gisa is run)
**Format**: TOML

## Full Configuration Example

```toml
# gisa.config.toml

# Base directory for all cloned repos
base_path = "~/github"

# Directory structure pattern
# {org} = organization name or GitHub username for personal repos
# {repo} = repository name
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

# Filter by visibility (future V2)
# visibility = "all"  # "all", "public", "private"

# Filter by specific orgs (future V2)
# orgs = ["org-a", "org-b"]

# Exclude specific repos (future V2)
# exclude_repos = ["org/repo-to-skip"]
```

## Configuration Options

### Core Settings

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `base_path` | string | `"~/github"` | Root directory for cloned repos |
| `structure` | string | `"{org}/{repo}"` | Directory structure pattern |
| `concurrency` | integer | `4` | Parallel operations (1-16) |
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

## CLI Flag Overrides

All config options can be overridden via CLI flags:

```bash
# Override concurrency
gisa clone ~/github --jobs 8

# Override sync mode
gisa sync ~/github --mode pull

# Override filters
gisa clone ~/github --include-archived --include-forks

# Shallow clone
gisa clone ~/github --depth 1

# Include submodules
gisa clone ~/github --recurse-submodules
```

**Precedence**: CLI flags > config file > defaults

## Minimal Config

For most users, a minimal config is sufficient:

```toml
# gisa.config.toml
base_path = "~/github"
```

All other options use sensible defaults.

## Config Initialization

```bash
# Create default config file
gisa init

# Creates gisa.config.toml with documented defaults
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
