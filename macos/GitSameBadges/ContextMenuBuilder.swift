// ContextMenuBuilder.swift
// Builds the right-click context menu for git repository folders and org folders.
// Everything lives under a single top-level "Git-Same" item. Inside, data is
// grouped into four sub-submenus: Organization, Workspace, Repositories /
// Repository, Repository list / Repository details — followed by the
// last-scan timestamp and the action items.

import Cocoa

enum ContextMenuBuilder {
    /// Build the context menu for a repository folder.
    static func build(for repo: FinderRepoStatus,
                      workspaceInfo: FinderWorkspaceInfo?,
                      timestamp: String?,
                      socketClient: SocketClient) -> NSMenu {
        // Ambient repos ship with `.gray` and no git details. Fire a targeted
        // REFRESH so the monitor runs a full scan_repo on this path; the
        // StatusReader file watcher will then replace the gray badge with a
        // semantic color within the next Finder tick.
        if repo.badge == .gray {
            socketClient.send("REFRESH \(repo.path)") { _ in }
        }
        let menu = NSMenu(title: "Git-Same")
        menu.addItem(parentItem(badge: repo.badge,
                                submenu: repoRoot(repo: repo,
                                                  workspaceInfo: workspaceInfo,
                                                  timestamp: timestamp)))
        return menu
    }

    /// Build the context menu for an organization (or user) folder.
    static func build(for org: OrgFolderInfo,
                      repos: [FinderRepoStatus],
                      workspaceInfo: FinderWorkspaceInfo?,
                      timestamp: String?) -> NSMenu {
        let menu = NSMenu(title: "Git-Same")
        menu.addItem(parentItem(badge: nil,
                                submenu: orgRoot(org: org, repos: repos,
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

    // MARK: - Repo root submenu

    private static func repoRoot(repo: FinderRepoStatus,
                                 workspaceInfo: FinderWorkspaceInfo?,
                                 timestamp: String?) -> NSMenu {
        let root = NSMenu()

        root.addItem(submenuRow(title: "Organization",
                                content: repoOrganizationSubmenu(repo: repo)))
        root.addItem(submenuRow(title: "Workspace",
                                content: repoWorkspaceSubmenu(workspaceInfo: workspaceInfo)))
        root.addItem(submenuRow(title: "Repository",
                                content: repoSelfSubmenu(repo: repo)))
        root.addItem(submenuRow(title: "Repository details",
                                content: repoDetailsSubmenu(repo: repo)))

        if let stamp = formatTimestamp(timestamp) {
            root.addItem(NSMenuItem.separator())
            root.addItem(infoItem("Last scan: \(stamp)"))
        }

        root.addItem(NSMenuItem.separator())
        root.addItem(NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        ))
        root.addItem(NSMenuItem(
            title: "Open in Terminal",
            action: #selector(FinderSync.openInTerminal(_:)),
            keyEquivalent: ""
        ))
        return root
    }

    private static func repoOrganizationSubmenu(repo: FinderRepoStatus) -> NSMenu {
        let sub = NSMenu()
        if let org = repo.org {
            sub.addItem(infoItem("Org: \(org)"))
        } else {
            sub.addItem(infoItem("(no org)"))
        }
        return sub
    }

    private static func repoWorkspaceSubmenu(workspaceInfo: FinderWorkspaceInfo?) -> NSMenu {
        let sub = NSMenu()
        if let ws = workspaceInfo {
            sub.addItem(infoItem("Name: \(ws.name)"))
            sub.addItem(infoItem("Root: \(ws.root)"))
        } else {
            sub.addItem(infoItem("(no workspace)"))
        }
        return sub
    }

    private static func repoSelfSubmenu(repo: FinderRepoStatus) -> NSMenu {
        let sub = NSMenu()

        sub.addItem(statusRow(badge: repo.badge))

        if repo.badge == .gray {
            sub.addItem(ambientHintItem())
            sub.addItem(NSMenuItem.separator())
            sub.addItem(infoItem("Path: \(repo.path)"))
            return sub
        }

        sub.addItem(NSMenuItem.separator())

        sub.addItem(infoItem("Branch: \(repo.currentBranch)"))
        sub.addItem(infoItem(upstreamLine(for: repo)))
        if let defaultBranch = repo.defaultBranch, defaultBranch != repo.currentBranch {
            sub.addItem(infoItem("Default: \(defaultBranch)"))
        }
        sub.addItem(infoItem(indexLine(for: repo)))
        sub.addItem(infoItem(workdirLine(for: repo)))
        if repo.stashCount > 0 {
            sub.addItem(infoItem("Stashes: \(repo.stashCount)"))
        }
        if repo.hasImportantIgnoredFiles {
            sub.addItem(importantIgnoredItem(repo: repo))
        }

        sub.addItem(NSMenuItem.separator())

        sub.addItem(infoItem("Commits: \(formattedCommits(repo.commitCount))"))
        sub.addItem(infoItem("Path: \(repo.path)"))

        return sub
    }

    private static func repoDetailsSubmenu(repo: FinderRepoStatus) -> NSMenu {
        let sub = NSMenu()
        if !repo.branches.isEmpty {
            let label = repo.allBranchesSynced
                ? "Branches \(repo.branches.count) (\u{2713} all synced)"
                : "Branches \(repo.branches.count) (some out of sync)"
            sub.addItem(branchesItem(title: label, branches: repo.branches,
                                     currentBranch: repo.currentBranch))
        }
        if !repo.remotes.isEmpty {
            sub.addItem(remotesItem(remotes: repo.remotes))
        }
        if !repo.worktrees.isEmpty {
            let label = repo.allWorktreesSynced
                ? "Worktrees \(repo.worktrees.count) (\u{2713} all synced)"
                : "Worktrees \(repo.worktrees.count) (some out of sync)"
            sub.addItem(worktreesItem(title: label, worktrees: repo.worktrees))
        }
        if sub.items.isEmpty {
            sub.addItem(infoItem("(no branches, remotes, or worktrees)"))
        }
        return sub
    }

    // MARK: - Org root submenu

    private static func orgRoot(org: OrgFolderInfo,
                                repos: [FinderRepoStatus],
                                workspaceInfo: FinderWorkspaceInfo?,
                                timestamp: String?) -> NSMenu {
        let root = NSMenu()

        root.addItem(submenuRow(title: "Organization",
                                content: orgOrganizationSubmenu(org: org)))
        root.addItem(submenuRow(title: "Workspace",
                                content: workspaceSubmenu(workspaceInfo: workspaceInfo,
                                                          currentOrg: org.org)))
        root.addItem(submenuRow(title: "Repositories (\(repos.count))",
                                content: orgAggregateSubmenu(repos: repos)))
        root.addItem(submenuRow(title: "Repository list",
                                content: orgRepoListSubmenu(repos: repos)))

        if let stamp = formatTimestamp(timestamp) {
            root.addItem(NSMenuItem.separator())
            root.addItem(infoItem("Last scan: \(stamp)"))
        }

        root.addItem(NSMenuItem.separator())
        root.addItem(NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        ))
        return root
    }

    private static func orgOrganizationSubmenu(org: OrgFolderInfo) -> NSMenu {
        let sub = NSMenu()
        sub.addItem(infoItem("Owner: \(org.org)"))
        if let typeLabel = ownerTypeLabel(org.ownerType) {
            sub.addItem(infoItem("Type: \(typeLabel)"))
        }
        sub.addItem(infoItem("Path: \(org.path)"))
        return sub
    }

    private static func workspaceSubmenu(workspaceInfo: FinderWorkspaceInfo?,
                                         currentOrg: String?) -> NSMenu {
        let sub = NSMenu()
        guard let ws = workspaceInfo else {
            sub.addItem(infoItem("(no workspace)"))
            return sub
        }
        sub.addItem(infoItem("Name: \(ws.name)"))
        sub.addItem(infoItem("Root: \(ws.root)"))
        sub.addItem(infoItem("Orgs in workspace: \(ws.orgs.count)"))
        if !ws.orgs.isEmpty {
            let orgsItem = NSMenuItem(title: "All orgs", action: nil, keyEquivalent: "")
            let orgsSubmenu = NSMenu()
            for name in ws.orgs.sorted() {
                let marker = (name == currentOrg) ? "\u{2713} " : "  "
                orgsSubmenu.addItem(infoItem("\(marker)\(name)"))
            }
            orgsItem.submenu = orgsSubmenu
            sub.addItem(orgsItem)
        }
        return sub
    }

    private static func orgAggregateSubmenu(repos: [FinderRepoStatus]) -> NSMenu {
        let sub = NSMenu()
        if repos.isEmpty {
            sub.addItem(infoItem("(no repositories)"))
            return sub
        }

        let counts = badgeCounts(for: repos)
        sub.addItem(infoItem("Repos: \(repos.count)"))
        sub.addItem(infoItem(
            "\u{1F7E2} \(counts.green)  |  \u{1F535} \(counts.blue)  |  "
            + "\u{1F7E0} \(counts.orange)  |  \u{1F534} \(counts.red)"
            + "  |  \u{26AB} \(counts.gray)"
        ))

        let totals = aggregate(repos: repos)
        sub.addItem(infoItem("Total commits: \(formattedCommits(totals.commits))"))
        if totals.staged > 0 || totals.unstaged > 0 || totals.untracked > 0 {
            sub.addItem(infoItem(
                "Uncommitted \u{2014} staged: \(totals.staged), "
                + "unstaged: \(totals.unstaged), untracked: \(totals.untracked)"
            ))
        }
        if totals.ahead > 0 || totals.behind > 0 {
            sub.addItem(infoItem(
                "Total ahead: \(totals.ahead)  |  behind: \(totals.behind)"
            ))
        }
        if totals.stashes > 0 {
            sub.addItem(infoItem("Total stashes: \(totals.stashes)"))
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
            sub.addItem(warnItem)
        }
        return sub
    }

    private static func orgRepoListSubmenu(repos: [FinderRepoStatus]) -> NSMenu {
        let sub = NSMenu()
        if repos.isEmpty {
            sub.addItem(infoItem("(no repositories)"))
            return sub
        }
        for r in repos.sorted(by: { repoBasename($0) < repoBasename($1) }) {
            let line = "\(badgeEmoji(r.badge)) \(repoBasename(r)) [\(r.currentBranch)]"
            let row = NSMenuItem(title: line, action: nil, keyEquivalent: "")
            row.submenu = repoSelfSubmenu(repo: r)
            sub.addItem(row)
        }
        return sub
    }

    // MARK: - Repository submenu helpers

    private static func statusRow(badge: Badge) -> NSMenuItem {
        let title = "\(badgeEmoji(badge))  \(badgeMeaning(badge))"
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: NSFont.systemFontSize, weight: .semibold)
        ]
        item.attributedTitle = NSAttributedString(string: title, attributes: attrs)
        return item
    }

    private static func ambientHintItem() -> NSMenuItem {
        let title = "Refreshing in background\u{2026}"
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        let base = NSFont.systemFont(ofSize: NSFont.systemFontSize)
        let italic = NSFont(descriptor: base.fontDescriptor.withSymbolicTraits(.italic),
                            size: NSFont.systemFontSize) ?? base
        let attrs: [NSAttributedString.Key: Any] = [
            .font: italic,
            .foregroundColor: NSColor.secondaryLabelColor
        ]
        item.attributedTitle = NSAttributedString(string: title, attributes: attrs)
        return item
    }

    private static func upstreamLine(for repo: FinderRepoStatus) -> String {
        let currentInfo = repo.branches.first { $0.name == repo.currentBranch }
        guard let upstream = currentInfo?.upstream else {
            return "Upstream: none"
        }
        let state: String
        if repo.ahead > 0 && repo.behind > 0 {
            state = "\(repo.ahead) ahead, \(repo.behind) behind"
        } else if repo.ahead > 0 {
            state = "\(repo.ahead) ahead"
        } else if repo.behind > 0 {
            state = "\(repo.behind) behind"
        } else {
            state = "synced"
        }
        return "Upstream: \(upstream) (\(state))"
    }

    private static func indexLine(for repo: FinderRepoStatus) -> String {
        if repo.stagedCount == 0 {
            return "Index: clean"
        }
        return "Index: \(repo.stagedCount) staged"
    }

    private static func workdirLine(for repo: FinderRepoStatus) -> String {
        if repo.unstagedCount == 0 && repo.untrackedCount == 0 {
            return "Workdir: clean"
        }
        var parts: [String] = []
        if repo.unstagedCount > 0 { parts.append("\(repo.unstagedCount) unstaged") }
        if repo.untrackedCount > 0 { parts.append("\(repo.untrackedCount) untracked") }
        return "Workdir: " + parts.joined(separator: " \u{00B7} ")
    }

    private static func importantIgnoredItem(repo: FinderRepoStatus) -> NSMenuItem {
        let patterns = repo.importantIgnoredFiles ?? []
        let title = "\u{26A0}  Important ignored files (\(patterns.count))"
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")

        let attributed = NSMutableAttributedString(string: title)
        let warnRange = (title as NSString).range(of: "\u{26A0}")
        if warnRange.location != NSNotFound {
            attributed.addAttribute(.foregroundColor,
                                    value: NSColor.systemYellow,
                                    range: warnRange)
        }
        item.attributedTitle = attributed

        if !patterns.isEmpty {
            let sub = NSMenu()
            for p in patterns {
                sub.addItem(infoItem(p))
            }
            item.submenu = sub
        } else {
            item.isEnabled = false
        }
        return item
    }

    // MARK: - Branches / Remotes / Worktrees rows

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
            if let upstream = branch.upstream {
                let detail = NSMenu()
                detail.addItem(infoItem("Upstream: \(upstream)"))
                detail.addItem(infoItem("Ahead: \(branch.ahead)  |  Behind: \(branch.behind)"))
                detail.addItem(infoItem(branch.synced ? "Synced" : "Out of sync"))
                row.submenu = detail
            } else {
                row.isEnabled = false
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

    private static func submenuRow(title: String, content: NSMenu) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.submenu = content
        return item
    }

    private static func badgeEmoji(_ badge: Badge) -> String {
        switch badge {
        case .green: return "\u{1F7E2}"  // green circle
        case .blue: return "\u{1F535}"   // blue circle
        case .orange: return "\u{1F7E0}" // orange circle
        case .red: return "\u{1F534}"    // red circle
        case .gray: return "\u{26AB}"    // black/gray circle
        }
    }

    private static func badgeMeaning(_ badge: Badge) -> String {
        switch badge {
        case .green: return "Synced"
        case .blue: return "Has Local Config"
        case .orange: return "Partially Synced"
        case .red: return "Uncommitted Changes"
        case .gray: return "Git Repository"
        }
    }

    private static func ownerTypeLabel(_ ownerType: OwnerType?) -> String? {
        switch ownerType {
        case .some(.user): return "User"
        case .some(.organization): return "Organization"
        case .some(.unknown), .none: return nil
        }
    }

    private struct BadgeCounts {
        var green = 0
        var blue = 0
        var orange = 0
        var red = 0
        var gray = 0
    }

    private static func badgeCounts(for repos: [FinderRepoStatus]) -> BadgeCounts {
        var counts = BadgeCounts()
        for r in repos {
            switch r.badge {
            case .green: counts.green += 1
            case .blue: counts.blue += 1
            case .orange: counts.orange += 1
            case .red: counts.red += 1
            case .gray: counts.gray += 1
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

    private static let commitsFormatter: NumberFormatter = {
        let f = NumberFormatter()
        f.numberStyle = .decimal
        return f
    }()

    private static func formattedCommits(_ count: UInt64) -> String {
        return commitsFormatter.string(from: NSNumber(value: count)) ?? "\(count)"
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
