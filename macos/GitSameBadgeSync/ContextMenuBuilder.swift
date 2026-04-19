// ContextMenuBuilder.swift
// Builds the right-click context menu for git repository folders and org folders.
// Everything lives under a single top-level "Git-Same" item so the Finder
// context menu stays clean.

import Cocoa

enum ContextMenuBuilder {
    /// Build the context menu for a repository.
    /// Returns an NSMenu with exactly one item: a `Git-Same` row whose
    /// submenu contains all the data and actions.
    static func build(for repo: FinderRepoStatus, socketClient: SocketClient) -> NSMenu {
        let menu = NSMenu(title: "GitSameBadge")
        menu.addItem(parentItem(badge: repo.badge, submenu: repoSubmenu(for: repo)))
        return menu
    }

    /// Build the context menu for an organization folder.
    static func build(for org: OrgFolderInfo) -> NSMenu {
        let menu = NSMenu(title: "GitSameBadge")
        menu.addItem(parentItem(badge: nil, submenu: orgSubmenu(for: org)))
        return menu
    }

    // MARK: - Parent item

    private static func parentItem(badge: Badge?, submenu: NSMenu) -> NSMenuItem {
        let prefix: String
        if let badge = badge {
            switch badge {
            case .green: prefix = "\u{1F7E2} "  // green circle
            case .blue: prefix = "\u{1F535} "   // blue circle
            case .orange: prefix = "\u{1F7E0} " // orange circle
            case .red: prefix = "\u{1F534} "    // red circle
            }
        } else {
            prefix = "\u{1F7E3} " // purple circle for org folders
        }

        let item = NSMenuItem(title: "\(prefix)Git-Same", action: nil, keyEquivalent: "")
        item.submenu = submenu
        return item
    }

    // MARK: - Repo submenu

    private static func repoSubmenu(for repo: FinderRepoStatus) -> NSMenu {
        let submenu = NSMenu()

        var headerAdded = false
        if let workspace = repo.workspace {
            submenu.addItem(infoItem("Workspace: \(workspace)"))
            headerAdded = true
        }
        if let org = repo.org {
            submenu.addItem(infoItem("Org: \(org)"))
            headerAdded = true
        }
        if headerAdded {
            submenu.addItem(NSMenuItem.separator())
        }

        submenu.addItem(infoItem("Branch: \(repo.currentBranch)"))
        if let defaultBranch = repo.defaultBranch, defaultBranch != repo.currentBranch {
            submenu.addItem(infoItem("Default: \(defaultBranch)"))
        }

        if repo.ahead > 0 || repo.behind > 0 {
            submenu.addItem(infoItem("Ahead: \(repo.ahead)  |  Behind: \(repo.behind)"))
        }

        submenu.addItem(infoItem("Commits: \(repo.commitCount)"))
        submenu.addItem(infoItem("Staged: \(repo.stagedCount)  |  Unstaged: \(repo.unstagedCount)"))
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
                ? "Branches (\u{2713} all synced)"
                : "Branches (some out of sync)"
            submenu.addItem(branchesItem(title: label, branches: repo.branches,
                                         currentBranch: repo.currentBranch))
        }

        if !repo.remotes.isEmpty {
            submenu.addItem(remotesItem(remotes: repo.remotes))
        }

        if !repo.worktrees.isEmpty {
            let label = repo.allWorktreesSynced
                ? "Worktrees (\u{2713} all synced)"
                : "Worktrees (some out of sync)"
            submenu.addItem(worktreesItem(title: label, worktrees: repo.worktrees))
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

    private static func orgSubmenu(for org: OrgFolderInfo) -> NSMenu {
        let submenu = NSMenu()
        submenu.addItem(infoItem("Org: \(org.org)"))
        submenu.addItem(infoItem("Workspace: \(org.workspace)"))
        submenu.addItem(infoItem("Path: \(org.path)"))
        submenu.addItem(NSMenuItem.separator())
        submenu.addItem(NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        ))
        return submenu
    }

    // MARK: - Sub-submenus

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
            sub.addItem(infoItem("\(checkmark)\(branch.name) \(syncStatus)"))
        }
        item.submenu = sub
        return item
    }

    private static func remotesItem(remotes: [FinderRemoteInfo]) -> NSMenuItem {
        let item = NSMenuItem(title: "Remotes", action: nil, keyEquivalent: "")
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

    private static func infoItem(_ title: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        return item
    }
}
