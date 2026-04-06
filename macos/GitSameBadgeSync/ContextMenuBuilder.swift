// ContextMenuBuilder.swift
// Builds the right-click context menu for git repository folders.

import Cocoa

enum ContextMenuBuilder {
    /// Build the context menu for a repository.
    static func build(for repo: FinderRepoStatus, socketClient: SocketClient) -> NSMenu {
        let menu = NSMenu(title: "GitSameBadge")

        // Header with badge indicator
        let badgeEmoji: String
        switch repo.badge {
        case .green: badgeEmoji = "\u{1F7E2}" // green circle
        case .blue: badgeEmoji = "\u{1F535}"   // blue circle
        case .orange: badgeEmoji = "\u{1F7E0}" // orange circle
        case .red: badgeEmoji = "\u{1F534}"    // red circle
        }

        let header = NSMenuItem(title: "\(badgeEmoji) GitSameBadge", action: nil, keyEquivalent: "")
        header.isEnabled = false
        menu.addItem(header)
        menu.addItem(NSMenuItem.separator())

        // Branch info
        menu.addItem(infoItem("Branch: \(repo.currentBranch)"))
        menu.addItem(infoItem("Commits: \(repo.commitCount)"))

        // Staged / Unstaged
        let changesLine = "Staged: \(repo.stagedCount)  |  Unstaged: \(repo.unstagedCount)"
        menu.addItem(infoItem(changesLine))
        if repo.untrackedCount > 0 {
            menu.addItem(infoItem("Untracked: \(repo.untrackedCount)"))
        }
        if repo.stashCount > 0 {
            menu.addItem(infoItem("Stashes: \(repo.stashCount)"))
        }

        // Branches submenu
        if !repo.branches.isEmpty {
            menu.addItem(NSMenuItem.separator())
            let branchesItem = NSMenuItem(title: "Branches", action: nil, keyEquivalent: "")
            let branchesSubmenu = NSMenu()

            for branch in repo.branches {
                let checkmark = (branch.name == repo.currentBranch) ? "\u{2713} " : "  "
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

                let title = "\(checkmark)\(branch.name) \(syncStatus)"
                branchesSubmenu.addItem(infoItem(title))
            }

            branchesItem.submenu = branchesSubmenu
            menu.addItem(branchesItem)
        }

        // Remotes submenu
        if !repo.remotes.isEmpty {
            let remotesItem = NSMenuItem(title: "Remotes", action: nil, keyEquivalent: "")
            let remotesSubmenu = NSMenu()
            for remote in repo.remotes {
                remotesSubmenu.addItem(infoItem("\(remote.name): \(remote.url)"))
            }
            remotesItem.submenu = remotesSubmenu
            menu.addItem(remotesItem)
        }

        // Worktrees submenu
        if !repo.worktrees.isEmpty {
            let worktreesItem = NSMenuItem(title: "Worktrees", action: nil, keyEquivalent: "")
            let worktreesSubmenu = NSMenu()
            for wt in repo.worktrees {
                let syncMark = wt.synced ? "\u{2713}" : "\u{2717}"
                let branch = wt.branch ?? "detached"
                worktreesSubmenu.addItem(infoItem("\(wt.path) (\(branch)) \(syncMark)"))
            }
            worktreesItem.submenu = worktreesSubmenu
            menu.addItem(worktreesItem)
        }

        // Actions
        menu.addItem(NSMenuItem.separator())

        let refreshItem = NSMenuItem(
            title: "\u{21BB} Refresh Status",
            action: #selector(FinderSync.refreshStatus(_:)),
            keyEquivalent: ""
        )
        menu.addItem(refreshItem)

        let terminalItem = NSMenuItem(
            title: "Open in Terminal",
            action: #selector(FinderSync.openInTerminal(_:)),
            keyEquivalent: ""
        )
        menu.addItem(terminalItem)

        return menu
    }

    /// Create a disabled info item (non-clickable label).
    private static func infoItem(_ title: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        return item
    }
}
