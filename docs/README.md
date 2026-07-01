# Git-Same

Mirror your GitHub org structure to the local filesystem: parallel clone, incremental sync, CLI, TUI dashboard, and a macOS Tauri app with Finder badges and workspace folder icons.

| ![TUI Dashboard (dark mode)](screenshots/tui-dashboard-dark.png) |
|:---:|

| ![TUI Dashboard (light mode)](screenshots/tui-dashboard-light.png) |
|:---:|

## What It Does

```
+--------------------------------------+------------+------------------------------------+
│ GITHUB PLATFORM                      │    Sync    │ LOCAL FILE SYSTEM                  │
│ github.com                           │            │ /users/m/same-github/              │
+--------------------------------------+------------+------------------------------------+
│ github.com/manuelgruber              │   <<==>>   │ manuelgruber/                      │
│ github.com/manuelgruber/.github      │   <<==>>   │ manuelgruber/.github/              │
│ github.com/manuelgruber/dotfiles     │   <<==>>   │ manuelgruber/dotfiles/             │
+--------------------------------------+------------+------------------------------------+
│ github.com/zaai-com                  │   <<==>>   │ zaai-com/                          │
│ github.com/zaai-com/powernight       │   <<==>>   │ zaai-com/powernight/               │
│ github.com/zaai-com/clean-autofill   │   <<==>>   │ zaai-com/clean-autofill/           │
│ github.com/zaai-com/git-same         │   <<==>>   │ zaai-com/git-same/                 │
│ github.com/zaai-com/jekyll-aeo       │   <<==>>   │ zaai-com/jekyll-aeo/               │
+--------------------------------------+------------+------------------------------------+
│ github.com/company1                  │   <<==>>   │ company1/                          │
│ github.com/company1/example.ai       │   <<==>>   │ company1/example.ai/               │
+--------------------------------------+------------+------------------------------------+
│ 3 orgs · 7 repos                     │            │ 3 dirs · 7 repos                   │
+--------------------------------------+------------+------------------------------------+
```

One command discovers every repo across your GitHub orgs and mirrors them locally, cloning new repos in parallel, fetching updates for existing ones, and skipping repos with uncommitted changes. On macOS, the cask also installs `Git-Same.app`, FinderSync badges, and a monitor LaunchAgent so workspace status is visible in Finder.

## Installation

**macOS** (signed + notarized cask):

```bash
brew install --cask zaai-com/tap/git-same
```

**Linux and headless macOS** (formula):

```bash
brew install zaai-com/tap/git-same-cli
```

> The macOS cask ships the Tauri GUI app, Finder badges, monitor LaunchAgent, and CLI aliases. `brew upgrade --cask git-same` keeps the app and command-line tools current together.

<details>
<summary>Other installation methods</summary>

#### From crates.io

```bash
cargo install git-same
```

#### GitHub Releases

Download pre-built assets from [GitHub Releases](https://github.com/zaai-com/git-same/releases). Linux and headless macOS CLI builds are `.tar.gz` archives named `git-same-<version>-<target-triple>.tar.gz` (for example, `git-same-3.1.0-aarch64-apple-darwin.tar.gz`). macOS GUI builds are signed and notarized DMGs named `git-same-<version>-<arch>.dmg`.

</details>

## Quick Start

**Interactive (TUI)**

```bash
git-same
```

Launches the full terminal UI with dashboard, sync, status, and workspace management, all via keyboard shortcuts.

**CLI**

```bash
git-same init          # 1. Create user config
git-same setup         # 2. Configure workspace (interactive wizard)
git-same sync          # 3. Clone new repos, fetch/pull existing
git-same status        # 4. Check repo status across orgs
```

## Commands

| Command | Description |
|---------|-------------|
| `git-same init` | Create config file with sensible defaults |
| `git-same setup` | Interactive wizard to configure a workspace |
| `git-same sync` | Discover, clone new, fetch/pull existing repos |
| `git-same status` | Show git status across all local repos |
| `git-same workspace` | List workspaces, set default |
| `git-same reset` | Remove all config, workspaces, and cache |
| `git-same monitor` | Run or inspect the background status monitor |
| `git-same refresh` | Ask the running monitor to rescan immediately |
| `git-same scan` | Discover repos without cloning or syncing |

### `git-same init`

Initialize git-same configuration:

```bash
git-same init [-p <config-path>] [-f | --force]
```

Creates a config file at `~/.config/git-same/config.toml` with sensible defaults.

### `git-same setup`

Configure a workspace (interactive wizard):

```bash
git-same setup [--name <NAME>]
```

Walks through provider selection, authentication, org filters, and base path.

| ![Setup Wizard: org selection](screenshots/tui-setup-workspace.png) |
|:---:|

### `git-same sync`

Sync repositories: discover, clone new, fetch/pull existing:

```bash
git-same sync [OPTIONS]

Options:
  -w, --workspace <WORKSPACE> Workspace to sync (path or unique folder name)
      --pull                  Use pull instead of fetch for existing repos
  -n, --dry-run               Show what would be done
  -c, --concurrency <N>       Number of parallel operations (1-32)
      --refresh               Force re-discovery (ignore cache)
      --no-skip-uncommitted         Don't skip repos with uncommitted changes
```

**Discovering repos**

| ![Sync: discovering repos](screenshots/tui-sync-discovery.png) |
|:---:|

**Cloning & fetching**

| ![Sync: cloning and fetching](screenshots/tui-sync-running.png) |
|:---:|

**Completed**

| ![Sync: completed](screenshots/tui-sync-completed.png) |
|:---:|

### `git-same status`

Show status of local repositories:

```bash
git-same status [OPTIONS]

Options:
  -w, --workspace <WORKSPACE> Workspace to check (path or unique folder name)
  -o, --org <ORG>...          Filter by organization (repeatable)
  -d, --uncommitted                 Show only repositories with uncommitted changes
  -b, --behind                Show only repositories behind upstream
      --detailed              Show detailed status information
```

| ![CLI status output](screenshots/cli-status.png) |
|:---:|

### `git-same workspace`

Manage workspaces:

```bash
git-same workspace list              # List configured workspaces
git-same workspace default [WORKSPACE] # Set default workspace (path or unique folder name)
git-same workspace default --clear   # Clear default workspace
```

### `git-same reset`

Remove all config, workspaces, and cache:

```bash
git-same reset [-f | --force]
```

### `git-same monitor`

Run the status monitor used by the macOS Finder extension and app:

```bash
git-same monitor [OPTIONS]

Options:
      --foreground       Legacy flag; the monitor runs in the foreground today
      --interval <SECS>  Polling interval, overriding config.toml
      --status           Show monitor status
      --stop             Stop a running monitor
```

The Homebrew cask installs a LaunchAgent that runs the monitor for you. Non-cask installs can run `git-same monitor` directly when Finder badge or app status updates are needed. The older `daemon` subcommand remains as a compatibility alias.

### `git-same refresh`

Ask the running monitor to rewrite Finder/app status immediately:

```bash
git-same refresh [--path <DIR>]
```

Use this after manually changing workspace folders, deleting repos, or debugging Finder badges. Without `--path`, every monitored workspace is refreshed.

### `git-same scan`

Scan a directory tree for existing workspace markers:

```bash
git-same scan [PATH] [--depth <N>] [--register]
```

Use `--register` to add discovered workspaces to your config automatically.

## Aliases

Git-Same installs multiple binary names so you can use whichever you prefer:

| Command    | Description                                    |
|------------|------------------------------------------------|
| `git-same` | Primary binary (always available)              |
| `gitsame`  | No-hyphen alias (symlink)                      |
| `gitsa`    | Short alias (symlink)                          |
| `gisa`     | Shortest alias (symlink)                       |
| `git same` | Git subcommand (requires git-same in PATH)     |

> **Install method differences:** Homebrew cask (`brew install --cask zaai-com/tap/git-same`) and the headless formula (`brew install zaai-com/tap/git-same-cli`) both install all aliases automatically. `cargo install git-same` installs only the primary binary.

All examples in this README use `git-same`, but any alias works interchangeably.

## macOS App and Finder Badges

The cask installs `Git-Same.app`, the CLI aliases, a FinderSync badge extension, and a monitor LaunchAgent. The app reads the same config as the CLI and shows workspace status from the monitor. Finder badges use the monitor's status file, and workspace root folders get a custom Git-Same folder icon unless `[ui] custom_folder_icon = false` is set.

Useful checks:

```bash
gisa monitor --status   # Check whether the monitor is running
gisa refresh            # Force an immediate status refresh
gisa monitor --stop     # Stop a running monitor
```

If Finder reserves badge space but no Git-Same badges render, check System Settings -> Login Items & Extensions. Google Drive's FinderSync extension can prevent peer badge images from appearing; disabling Google Drive's Finder extension has been the confirmed workaround in affected environments.

## Requirements

Git-Same depends on two external tools at runtime:

- **[`git`](https://git-scm.com/)**: Git-Same shells out to `git` for all repository operations (clone, fetch, pull, and status). Without it, no git operations can run.
- **[`gh`](https://cli.github.com/) (GitHub CLI)**: Git-Same calls `gh auth token` to obtain GitHub API tokens for repo discovery and `gh api user` to resolve your username. Without it, Git-Same cannot authenticate with the GitHub API.

### Installing and authenticating `gh`

```bash
# Install GitHub CLI
brew install gh  # macOS
# or: sudo apt install gh  # Ubuntu

# Authenticate
gh auth login

# Git-Same will now use your gh credentials
git-same sync
```

## TUI Mode

Running `git-same` without a subcommand launches the interactive terminal UI.

### Screens

| Screen | Purpose | Key bindings |
|--------|---------|-------------|
| **Dashboard** | Overview with status tabs, search, and sync actions | `s`: Start sync, `p`: Show sync progress, `t`: Refresh status, `w`: Workspaces, `e`/`i`: Settings, `/`: Search repositories, `[←]`/`[→]`: Switch tabs, `[↑]`/`[↓]`: Move within tab, `Enter`: Open selected repo folder |
| **Workspace Selector** | Pick, inspect, and create workspaces | `[←]`/`[→]` or `Tab`: Switch pane, `[↑]`/`[↓]`: Move or scroll, `Enter`: Select workspace or create new, `d`: Set default, `n`: New workspace, `c`: Expand/collapse config, `f`: Open folder |
| **Setup Wizard** | Interactive workspace configuration | Step-by-step prompts for requirements, provider, authentication, orgs, path, and confirmation |
| **Sync Progress** | Live sync progress and post-sync results | `p`/`Esc`: Hide or go back, `[←]`/`[→]`: Switch result filters when finished, `[↑]`/`[↓]`: Move or scroll, `a`: All, `u`: Updated, `f`: Failed, `x`: Skipped, `c`: Changelog, `h`: History, `Enter`: Expand selected repo commit detail |
| **Settings** | View requirements and adjust run options | `Tab`/`[↑]`/`[↓]`: Move, `c`: Open config folder, `d`: Toggle dry run, `m`: Toggle fetch/pull mode, `Esc`: Back |

## Examples

### Sync all repositories in default workspace

```bash
git-same sync
```

### Sync with pull mode for a specific workspace

```bash
git-same sync --workspace work --pull
```

### Check which repositories have uncommitted changes

```bash
git-same status --uncommitted
```

### Dry run to see what would be synced

```bash
git-same sync --dry-run
```

## License

MIT License - see [LICENSE](../LICENSE) for details

## [Roadmap](https://zaai.com/git-same/)

- [x] GitHub support
- [x] Parallel cloning
- [x] Smart filtering
- [x] Progress bars
- [x] Interactive TUI mode
- [x] Workspace management
- [ ] GitLab support
- [ ] Bitbucket support
- [ ] Repo groups
- [ ] Web dashboard

---

Building from source or contributing? See [DEVELOPMENT.md](DEVELOPMENT.md).
