// ContextMenuBuilder.swift
// Builds the right-click context menu for git repository folders and org folders.
// Everything lives under a single top-level "Git-Same" item so the Finder
// context menu stays clean.

import Cocoa

enum ContextMenuBuilder {
    /// Build the context menu for a repository.
    /// Returns an NSMenu with exactly one item: a `Git-Same` row whose
    /// submenu contains all the data and actions.
    static func build(for repo: FinderRepoStatus,
                      timestamp: String?,
                      socketClient: SocketClient) -> NSMenu {
        let menu = NSMenu(title: "GitSameBadge")
        menu.addItem(parentItem(badge: repo.badge,
                                submenu: repoSubmenu(for: repo, timestamp: timestamp)))
        return menu
    }

    /// Build the context menu for an organization (or user) folder.
    static func build(for org: OrgFolderInfo,
                      repos: [FinderRepoStatus],
                      workspaceInfo: FinderWorkspaceInfo?,
                      timestamp: String?) -> NSMenu {
        let menu = NSMenu(title: "GitSameBadge")
        menu.addItem(parentItem(badge: nil,
                                submenu: orgSubmenu(for: org, repos: repos,
                                                    workspaceInfo: workspaceInfo,
                                                    timestamp: timestamp)))
        return menu
    }

    // MARK: - Parent item

    private static func parentItem(badge: Badge?, submenu: NSMenu) -> NSMenuItem {
        let prefix = badge.map(badgeEmoji) ?? "\u{1F7E3}"
        let item = NSMenuItem(title: "\(prefix) Git-Same", action: nil, keyEquivalent: "")
        item.submenu = submenu
        return item
    }

    private static func badgeEmoji(_ badge: Badge) -> String {
        switch badge {
        case .green: return "\u{1F7E2}"  // green circle
        case .blue: return "\u{1F535}"   // blue circle
        case .orange: return "\u{1F7E0}" // orange circle
        case .red: return "\u{1F534}"    // red circle
        }
    }

    private static func badgeMeaning(_ badge: Badge) -> String {
        switch badge {
        case .green: return "Synced"
        case .blue: return "Has Local Config"
        case .orange: return "Partially Synced"
        case .red: return "Uncommitted Changes"
        }
    }

    // MARK: - Repo submenu

    private static func repoSubmenu(for repo: FinderRepoStatus, timestamp: String?) -> NSMenu {
        let submenu = NSMenu()

        submenu.addItem(infoItem("Status: \(badgeMeaning(repo.badge))"))
        if let workspace = repo.workspace {
            submenu.addItem(infoItem("Workspace: \(workspace)"))
        }
        if let org = repo.org {
            submenu.addItem(infoItem("Org: \(org)"))
        }
        submenu.addItem(infoItem("Path: \(repo.path)"))
        submenu.addItem(NSMenuItem.separator())

        submenu.addItem(infoItem("Branch: \(repo.currentBranch)"))
        if let defaultBranch = repo.defaultBranch, defaultBranch != repo.currentBranch {
            submenu.addItem(infoItem("Default: \(defaultBranch)"))
        }

        if repo.ahead > 0 || repo.behind > 0 {
            submenu.addItem(infoItem("Ahead: \(repo.ahead)  |  Behind: \(repo.behind)"))
        }

        submenu.addItem(infoItem("Commits: \(repo.commitCount)"))
        submenu.addItem(infoItem(
            "Staged: \(repo.stagedCount)  |  Unstaged: \(repo.unstagedCount)"
        ))
        if repo.untrackedCount > 0 {
            submenu.addItem(infoItem("Untracked: \(repo.untrackedCount)"))
        }
        if repo.stashCount > 0 {
            submenu.addItem(infoItem("Stashes: \(repo.stashCount)"))
        }

        if repo.hasImportantIgnoredFiles {
            let patterns = repo.importantIgnoredFiles ?? []
            let warnTitle = "\u{26A0} Important ignored files (\(patterns.count))"
            let warnItem = NSMenuItem(title: warnTitle, action: nil, keyEquivalent: "")
            if !patterns.isEmpty {
                let warnSubmenu = NSMenu()
                for pattern in patterns {
                    warnSubmenu.addItem(infoItem(pattern))
                }
                warnItem.submenu = warnSubmenu
            } else {
                warnItem.isEnabled = false
            }
            submenu.addItem(warnItem)
        }

        if !repo.branches.isEmpty {
            submenu.addItem(NSMenuItem.separator())
            let label = repo.allBranchesSynced
                ? "Branches \(repo.branches.count) (\u{2713} all synced)"
                : "Branches \(repo.branches.count) (some out of sync)"
            submenu.addItem(branchesItem(title: label, branches: repo.branches,
                                         currentBranch: repo.currentBranch))
        }

        if !repo.remotes.isEmpty {
            submenu.addItem(remotesItem(remotes: repo.remotes))
        }

        if !repo.worktrees.isEmpty {
            let label = repo.allWorktreesSynced
                ? "Worktrees \(repo.worktrees.count) (\u{2713} all synced)"
                : "Worktrees \(repo.worktrees.count) (some out of sync)"
            submenu.addItem(worktreesItem(title: label, worktrees: repo.worktrees))
        }

        if let stamp = formatTimestamp(timestamp) {
            submenu.addItem(NSMenuItem.separator())
            submenu.addItem(infoItem("Last scan: \(stamp)"))
        }

        submenu.addItem(NSMenuItem.separator())
        submenu.addItem(NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        ))
        submenu.addItem(NSMenuItem(
            title: "Open in Terminal",
            action: #selector(FinderSync.openInTerminal(_:)),
            keyEquivalent: ""
        ))

        return submenu
    }

    // MARK: - Org submenu

    private static func orgSubmenu(for org: OrgFolderInfo,
                                   repos: [FinderRepoStatus],
                                   workspaceInfo: FinderWorkspaceInfo?,
                                   timestamp: String?) -> NSMenu {
        let submenu = NSMenu()

        submenu.addItem(infoItem("Owner: \(org.org)"))
        submenu.addItem(infoItem("Workspace: \(org.workspace)"))
        submenu.addItem(infoItem("Path: \(org.path)"))

        submenu.addItem(NSMenuItem.separator())

        let counts = badgeCounts(for: repos)
        submenu.addItem(infoItem("Repos: \(repos.count)"))
        if !repos.isEmpty {
            submenu.addItem(infoItem(
                "\u{1F7E2} \(counts.green)  |  \u{1F535} \(counts.blue)  |  "
                + "\u{1F7E0} \(counts.orange)  |  \u{1F534} \(counts.red)"
            ))
        }

        let totals = aggregate(repos: repos)
        submenu.addItem(infoItem("Total commits: \(totals.commits)"))
        if totals.staged > 0 || totals.unstaged > 0 || totals.untracked > 0 {
            submenu.addItem(infoItem(
                "Uncommitted \u{2014} staged: \(totals.staged), "
                + "unstaged: \(totals.unstaged), untracked: \(totals.untracked)"
            ))
        }
        if totals.ahead > 0 || totals.behind > 0 {
            submenu.addItem(infoItem(
                "Total ahead: \(totals.ahead)  |  behind: \(totals.behind)"
            ))
        }
        if totals.stashes > 0 {
            submenu.addItem(infoItem("Total stashes: \(totals.stashes)"))
        }

        let secretRepos = repos.filter { $0.hasImportantIgnoredFiles }
        if !secretRepos.isEmpty {
            let warnTitle = "\u{26A0} Repos with sensitive files (\(secretRepos.count))"
            let warnItem = NSMenuItem(title: warnTitle, action: nil, keyEquivalent: "")
            let warnSubmenu = NSMenu()
            for r in secretRepos.sorted(by: { repoBasename($0) < repoBasename($1) }) {
                warnSubmenu.addItem(infoItem(repoBasename(r)))
            }
            warnItem.submenu = warnSubmenu
            submenu.addItem(warnItem)
        }

        if !repos.isEmpty {
            submenu.addItem(NSMenuItem.separator())
            submenu.addItem(reposItem(repos: repos))
        }

        if let ws = workspaceInfo {
            submenu.addItem(NSMenuItem.separator())
            submenu.addItem(infoItem("Workspace root: \(ws.root)"))
            submenu.addItem(infoItem("Orgs in workspace: \(ws.orgs.count)"))
            if !ws.orgs.isEmpty {
                let orgsItem = NSMenuItem(title: "All orgs in workspace",
                                          action: nil, keyEquivalent: "")
                let orgsSubmenu = NSMenu()
                for name in ws.orgs.sorted() {
                    let marker = (name == org.org) ? "\u{2713} " : "  "
                    orgsSubmenu.addItem(infoItem("\(marker)\(name)"))
                }
                orgsItem.submenu = orgsSubmenu
                submenu.addItem(orgsItem)
            }
        }

        if let stamp = formatTimestamp(timestamp) {
            submenu.addItem(NSMenuItem.separator())
            submenu.addItem(infoItem("Last scan: \(stamp)"))
        }

        submenu.addItem(NSMenuItem.separator())
        submenu.addItem(NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        ))
        return submenu
    }

    // MARK: - Sub-submenus

    private static func reposItem(repos: [FinderRepoStatus]) -> NSMenuItem {
        let item = NSMenuItem(title: "Repos (\(repos.count))",
                              action: nil, keyEquivalent: "")
        let sub = NSMenu()
        for r in repos.sorted(by: { repoBasename($0) < repoBasename($1) }) {
            let line = "\(badgeEmoji(r.badge)) \(repoBasename(r)) [\(r.currentBranch)]"
            let row = NSMenuItem(title: line, action: nil, keyEquivalent: "")
            row.submenu = repoMiniSubmenu(for: r)
            sub.addItem(row)
        }
        item.submenu = sub
        return item
    }

    private static func repoMiniSubmenu(for repo: FinderRepoStatus) -> NSMenu {
        let sub = NSMenu()
        sub.addItem(infoItem("Status: \(badgeMeaning(repo.badge))"))
        sub.addItem(infoItem("Branch: \(repo.currentBranch)"))
        if let def = repo.defaultBranch, def != repo.currentBranch {
            sub.addItem(infoItem("Default: \(def)"))
        }
        sub.addItem(infoItem("Commits: \(repo.commitCount)"))
        if repo.ahead > 0 || repo.behind > 0 {
            sub.addItem(infoItem("Ahead: \(repo.ahead)  |  Behind: \(repo.behind)"))
        }
        if repo.stagedCount > 0 || repo.unstagedCount > 0 || repo.untrackedCount > 0 {
            sub.addItem(infoItem(
                "Staged: \(repo.stagedCount)  |  Unstaged: \(repo.unstagedCount)"
                + "  |  Untracked: \(repo.untrackedCount)"
            ))
        }
        if repo.stashCount > 0 {
            sub.addItem(infoItem("Stashes: \(repo.stashCount)"))
        }
        if repo.hasImportantIgnoredFiles {
            sub.addItem(infoItem(
                "\u{26A0} Important ignored files: \((repo.importantIgnoredFiles ?? []).count)"
            ))
        }
        sub.addItem(infoItem("Path: \(repo.path)"))
        return sub
    }

    private static func branchesItem(title: String, branches: [FinderBranchInfo],
                                     currentBranch: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        let sub = NSMenu()
        for branch in branches {
            let checkmark = (branch.name == currentBranch) ? "\u{2713} " : "  "
            let syncStatus: String
            if branch.synced {
                syncStatus = "(synced)"
            } else if branch.ahead > 0 && branch.behind > 0 {
                syncStatus = "(ahead \(branch.ahead), behind \(branch.behind))"
            } else if branch.ahead > 0 {
                syncStatus = "(ahead \(branch.ahead))"
            } else if branch.behind > 0 {
                syncStatus = "(behind \(branch.behind))"
            } else if branch.upstream == nil {
                syncStatus = "(no upstream)"
            } else {
                syncStatus = ""
            }
            let row = NSMenuItem(title: "\(checkmark)\(branch.name) \(syncStatus)",
                                 action: nil, keyEquivalent: "")
            row.isEnabled = false
            if let upstream = branch.upstream {
                let detail = NSMenu()
                detail.addItem(infoItem("Upstream: \(upstream)"))
                detail.addItem(infoItem("Ahead: \(branch.ahead)  |  Behind: \(branch.behind)"))
                detail.addItem(infoItem(branch.synced ? "Synced" : "Out of sync"))
                row.submenu = detail
                row.isEnabled = true
            }
            sub.addItem(row)
        }
        item.submenu = sub
        return item
    }

    private static func remotesItem(remotes: [FinderRemoteInfo]) -> NSMenuItem {
        let item = NSMenuItem(title: "Remotes (\(remotes.count))",
                              action: nil, keyEquivalent: "")
        let sub = NSMenu()
        for remote in remotes {
            sub.addItem(infoItem("\(remote.name): \(remote.url)"))
        }
        item.submenu = sub
        return item
    }

    private static func worktreesItem(title: String,
                                      worktrees: [FinderWorktreeInfo]) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        let sub = NSMenu()
        for wt in worktrees {
            let syncMark = wt.synced ? "\u{2713}" : "\u{2717}"
            let branch = wt.branch ?? "detached"
            sub.addItem(infoItem("\(wt.path) (\(branch)) \(syncMark)"))
        }
        item.submenu = sub
        return item
    }

    // MARK: - Helpers

    private struct BadgeCounts {
        var green = 0
        var blue = 0
        var orange = 0
        var red = 0
    }

    private static func badgeCounts(for repos: [FinderRepoStatus]) -> BadgeCounts {
        var counts = BadgeCounts()
        for r in repos {
            switch r.badge {
            case .green: counts.green += 1
            case .blue: counts.blue += 1
            case .orange: counts.orange += 1
            case .red: counts.red += 1
            }
        }
        return counts
    }

    private struct AggregateTotals {
        var commits: UInt64 = 0
        var staged: Int = 0
        var unstaged: Int = 0
        var untracked: Int = 0
        var ahead: UInt32 = 0
        var behind: UInt32 = 0
        var stashes: Int = 0
    }

    private static func aggregate(repos: [FinderRepoStatus]) -> AggregateTotals {
        var t = AggregateTotals()
        for r in repos {
            t.commits += r.commitCount
            t.staged += r.stagedCount
            t.unstaged += r.unstagedCount
            t.untracked += r.untrackedCount
            t.ahead += r.ahead
            t.behind += r.behind
            t.stashes += r.stashCount
        }
        return t
    }

    private static func repoBasename(_ repo: FinderRepoStatus) -> String {
        return (repo.path as NSString).lastPathComponent
    }

    private static let isoFormatter: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return f
    }()

    private static let isoFormatterNoFractional: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime]
        return f
    }()

    private static let relativeFormatter: RelativeDateTimeFormatter = {
        let f = RelativeDateTimeFormatter()
        f.unitsStyle = .full
        return f
    }()

    private static let absoluteFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .short
        f.timeStyle = .medium
        return f
    }()

    private static func formatTimestamp(_ stamp: String?) -> String? {
        guard let stamp = stamp, !stamp.isEmpty else { return nil }
        let date = isoFormatter.date(from: stamp)
            ?? isoFormatterNoFractional.date(from: stamp)
        guard let date = date else { return stamp }
        let relative = relativeFormatter.localizedString(for: date, relativeTo: Date())
        let absolute = absoluteFormatter.string(from: date)
        return "\(relative) (\(absolute))"
    }

    private static func infoItem(_ title: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        return item
    }
}
