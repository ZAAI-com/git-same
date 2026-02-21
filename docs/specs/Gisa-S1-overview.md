# Git-Same Overview

Git-Same (also known as Gisa) is a CLI tool that mirrors GitHub organization and repository structure to the local filesystem.

## Problem

Developers who belong to multiple GitHub organizations and have access to dozens or hundreds of repositories lack a simple way to clone and maintain a local mirror of that structure. Manual cloning is tedious, and keeping repositories in sync requires visiting each one individually.

## Solution

Git-Same discovers all GitHub organizations and repositories a user has access to, then clones them into a configurable local directory structure. It also provides incremental sync operations (fetch/pull) and status reporting across all repositories.

## Key Features

- **Discovery**: Automatically finds all orgs and repos via the GitHub API
- **Multi-Provider Support**: GitHub and GitHub Enterprise (GitLab and Bitbucket planned)
- **Parallel Operations**: Concurrent cloning and syncing with configurable concurrency
- **Smart Filtering**: Filter by organization, exclude archived repos or forks
- **Incremental Sync**: Fetch or pull updates across all repositories
- **Caching**: Cache discovery results to avoid API rate limits
- **Progress Reporting**: Real-time progress bars and status updates
- **Shell Completions**: Bash, Zsh, Fish, PowerShell, Elvish

## Target Users

- Developers who belong to multiple GitHub organizations
- Teams that need to maintain local mirrors of org repositories
- Anyone who wants a structured local copy of their GitHub repos

## Scope

**In scope:**
- Repository discovery via provider APIs
- Cloning with configurable directory structure
- Sync operations (fetch, pull)
- Status reporting (dirty, behind upstream)
- Authentication via `gh` CLI, environment variables, or personal access tokens
- Configuration via TOML config file

**Out of scope:**
- Repository creation or management on GitHub
- Push operations
- Branch management
- Issue/PR workflows

## Binary Names

The tool installs four binary aliases:
- `git-same` (primary)
- `gitsame`
- `gitsa`
- `gisa`

## Technology

- **Language**: Rust (2021 edition)
- **Config**: TOML at `~/.config/git-same/config.toml`
- **Repository**: https://github.com/zaai-com/git-same
- **License**: MIT
