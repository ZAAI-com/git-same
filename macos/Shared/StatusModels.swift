// StatusModels.swift
// Codable types matching the monitor's finder-status.json schema.

import Foundation

/// Badge color indicating repository health.
///
/// `gray` marks ambient (non-workspace) repos that haven't been fully scanned
/// yet — they upgrade to a semantic color when the user opens their context
/// menu.
enum Badge: String, Codable {
    case green
    case blue
    case orange
    case red
    case gray
}

/// Branch sync status.
struct FinderBranchInfo: Codable {
    let name: String
    let upstream: String?
    let ahead: UInt32
    let behind: UInt32
    let synced: Bool
}

/// Remote info.
struct FinderRemoteInfo: Codable {
    let name: String
    let url: String
}

/// Worktree info.
struct FinderWorktreeInfo: Codable {
    let path: String
    let branch: String?
    let synced: Bool
}

/// Complete status for a single repository.
struct FinderRepoStatus: Codable {
    let path: String
    let workspace: String?
    let org: String?
    let badge: Badge
    let currentBranch: String
    let defaultBranch: String?
    let commitCount: UInt64
    let stagedCount: Int
    let unstagedCount: Int
    let untrackedCount: Int
    let ahead: UInt32
    let behind: UInt32
    let stashCount: Int
    let hasImportantIgnoredFiles: Bool
    let importantIgnoredFiles: [String]?
    let branches: [FinderBranchInfo]
    let allBranchesSynced: Bool
    let remotes: [FinderRemoteInfo]
    let worktrees: [FinderWorktreeInfo]
    let allWorktreesSynced: Bool

    enum CodingKeys: String, CodingKey {
        case path, workspace, org, badge
        case currentBranch = "current_branch"
        case defaultBranch = "default_branch"
        case commitCount = "commit_count"
        case stagedCount = "staged_count"
        case unstagedCount = "unstaged_count"
        case untrackedCount = "untracked_count"
        case ahead, behind
        case stashCount = "stash_count"
        case hasImportantIgnoredFiles = "has_important_ignored_files"
        case importantIgnoredFiles = "important_ignored_files"
        case branches
        case allBranchesSynced = "all_branches_synced"
        case remotes, worktrees
        case allWorktreesSynced = "all_worktrees_synced"
    }
}

/// Classification of the account that owns an org/user folder.
enum OwnerType: String, Codable {
    case user
    case organization
    case unknown
}

/// Organization or user folder inside a workspace.
struct OrgFolderInfo: Codable {
    let path: String
    let org: String
    let workspace: String
    let ownerType: OwnerType?

    enum CodingKeys: String, CodingKey {
        case path, org, workspace
        case ownerType = "owner_type"
    }
}

/// Workspace summary.
struct FinderWorkspaceInfo: Codable {
    let name: String
    let root: String
    let orgs: [String]
}

/// Top-level status file written by the monitor.
struct FinderStatus: Codable {
    let version: UInt32
    let timestamp: String
    let daemonPid: UInt32
    let workspaces: [FinderWorkspaceInfo]
    let customFolders: [String]?
    let repos: [FinderRepoStatus]
    let orgFolders: [OrgFolderInfo]?
    /// Union of workspace roots and ambient scan roots. The extension uses
    /// this as `FIFinderSyncController.directoryURLs` so Finder knows which
    /// folders to ask about.
    let monitoredRoots: [String]?
    /// `/Volumes/<name>` boot-volume-alias prefixes computed by the monitor.
    /// The extension uses these as pure-string prefixes to map alias-presented
    /// Finder paths back to canonical keys, so it never calls
    /// `resolvingSymlinksInPath()` and never touches the disk outside its
    /// app-group container.
    let bootVolumeAliases: [String]?

    enum CodingKeys: String, CodingKey {
        case version, timestamp
        case daemonPid = "daemon_pid"
        case workspaces
        case customFolders = "custom_folders"
        case repos
        case orgFolders = "org_folders"
        case monitoredRoots = "monitored_roots"
        case bootVolumeAliases = "boot_volume_aliases"
    }
}
