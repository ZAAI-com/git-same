# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-01-20

### Added

- Multiple command aliases installed by default:
  - `git-same` - Main command
  - `gitsame` - No hyphen variant
  - `gitsa` - Short form
  - `gisa` - Shortest variant
  - `git same` - Git subcommand support

- Complete feature set:
  - `init` - Initialize configuration
  - `clone` - Clone all repositories
  - `fetch` - Fetch updates without modifying working tree
  - `pull` - Pull updates to working tree
  - `status` - Show repository status
  - `completions` - Generate shell completions

- Multi-provider architecture:
  - GitHub support (github.com)
  - GitHub Enterprise support
  - GitLab support (coming soon)
  - Bitbucket support (coming soon)

- Smart filtering:
  - Filter by organization
  - Include/exclude archived repositories
  - Include/exclude forked repositories

- Parallel operations:
  - Concurrent cloning with configurable concurrency
  - Concurrent syncing (fetch/pull)
  - Progress bars with live updates

- Caching:
  - Cache discovery results to avoid API rate limits
  - Automatic cache invalidation
  - Optional cache refresh

- Authentication:
  - GitHub CLI (`gh`) integration
  - Environment variable tokens
  - Multi-provider auth configuration

- Configuration:
  - TOML-based configuration at `~/.config/git-same/config.toml`
  - Per-provider configuration
  - Flexible directory structure with placeholders

- Developer experience:
  - Shell completions (bash, zsh, fish, powershell, elvish)
  - Detailed error messages with suggestions
  - Dry-run mode for all operations
  - JSON output support
  - Verbose/quiet modes

### Changed

- Project renamed from "gisa" to "git-same"
- Config directory moved from `~/.config/gisa/` to `~/.config/git-same/`
- Repository URL: https://github.com/zaai-com/git-same

### Removed

- Removed `gs` alias to avoid conflicts with Ghostscript

### Technical

- 216 tests passing (192 unit + 8 doc + 16 integration)
- 0 clippy warnings
- Release binary size: 2.4 MB
- Cross-platform CI/CD (Linux, macOS, Windows)
- Built with Rust 2021 edition

## [0.1.0] - 2024-01-15

### Added

- Initial development version
- Basic GitHub cloning functionality
- Test-driven development foundation

[0.2.0]: https://github.com/zaai-com/git-same/releases/tag/v0.2.0
[0.1.0]: https://github.com/zaai-com/git-same/releases/tag/v0.1.0
