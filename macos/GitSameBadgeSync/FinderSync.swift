// FinderSync.swift
// macOS FinderSync extension that displays git status badges and context menus.

import Cocoa
import FinderSync

class FinderSync: FIFinderSync {

    let statusReader = StatusReader.shared
    let socketClient = SocketClient()

    override init() {
        super.init()

        // Register badge images
        BadgeManager.registerBadges()

        // Start watching the status file
        statusReader.onStatusUpdate = { [weak self] in
            self?.updateMonitoredDirectories()
        }
        statusReader.startWatching()
    }

    // MARK: - Monitored Directories

    private func updateMonitoredDirectories() {
        guard let status = statusReader.currentStatus else { return }

        var urls = Set<URL>()

        // Add workspace roots
        for workspace in status.workspaces {
            urls.insert(URL(fileURLWithPath: workspace.root))
        }

        // Add custom folders
        for folder in status.customFolders ?? [] {
            urls.insert(URL(fileURLWithPath: folder))
        }

        FIFinderSyncController.default().directoryURLs = urls
    }

    // MARK: - Badge Identifiers

    override func requestBadgeIdentifier(for url: URL) {
        let path = url.path

        // Check if it's an org folder
        if statusReader.isOrgFolder(path: path) {
            FIFinderSyncController.default().setBadgeIdentifier(
                GitSameBadgeConstants.BadgeID.org, for: url
            )
            return
        }

        // Check if it's a git repo
        if let repoStatus = statusReader.repoStatus(forPath: path) {
            let badgeID: String
            switch repoStatus.badge {
            case .green: badgeID = GitSameBadgeConstants.BadgeID.green
            case .blue: badgeID = GitSameBadgeConstants.BadgeID.blue
            case .orange: badgeID = GitSameBadgeConstants.BadgeID.orange
            case .red: badgeID = GitSameBadgeConstants.BadgeID.red
            }
            FIFinderSyncController.default().setBadgeIdentifier(badgeID, for: url)
        }
    }

    // MARK: - Toolbar

    override var toolbarItemName: String {
        return "GitSameBadge"
    }

    override var toolbarItemToolTip: String {
        return "GitSameBadge repository status"
    }

    override var toolbarItemImage: NSImage {
        return NSImage(named: NSImage.folderName)!
    }

    // MARK: - Context Menu

    override func menu(for menuKind: FIMenuKind) -> NSMenu {
        guard let targetURL = FIFinderSyncController.default().targetedURL() else {
            return NSMenu()
        }

        let path = targetURL.path

        if let repoStatus = statusReader.repoStatus(forPath: path) {
            return ContextMenuBuilder.build(for: repoStatus, socketClient: socketClient)
        }

        return NSMenu()
    }

    // MARK: - Context Menu Actions

    @objc func refreshStatus(_ sender: Any?) {
        socketClient.send("REFRESH_ALL") { _ in }
    }

    @objc func openInTerminal(_ sender: Any?) {
        guard let targetURL = FIFinderSyncController.default().targetedURL() else { return }
        NSWorkspace.shared.open(
            [targetURL],
            withApplicationAt: URL(fileURLWithPath: "/System/Applications/Utilities/Terminal.app"),
            configuration: NSWorkspace.OpenConfiguration()
        )
    }
}
