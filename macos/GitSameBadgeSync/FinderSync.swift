// FinderSync.swift
// macOS FinderSync extension that displays git status badges and context menus.

import Cocoa
import FinderSync
import os

private let gsbLog = OSLog(subsystem: "com.zaai.git-same.GitSameBadge.FinderSync", category: "ext")

class FinderSync: FIFinderSync {

    let statusReader = StatusReader.shared
    let socketClient = SocketClient()

    override init() {
        super.init()
        os_log("FinderSync init entered", log: gsbLog, type: .default)

        BadgeManager.registerBadges()

        statusReader.onStatusUpdate = { [weak self] in
            self?.updateMonitoredDirectories()
        }
        statusReader.startWatching()

        // StatusReader reads the file eagerly in its init but only invokes
        // onStatusUpdate on subsequent file-change events. Without this call,
        // directoryURLs stays empty until the daemon next rewrites the file,
        // so Finder never asks us for badges.
        updateMonitoredDirectories()
    }

    // MARK: - Monitored Directories

    private func updateMonitoredDirectories() {
        guard let status = statusReader.currentStatus else {
            os_log("updateMonitoredDirectories: no status yet", log: gsbLog, type: .default)
            return
        }

        var urls = Set<URL>()
        for workspace in status.workspaces {
            urls.insert(URL(fileURLWithPath: workspace.root))
        }
        for folder in status.customFolders ?? [] {
            urls.insert(URL(fileURLWithPath: folder))
        }

        FIFinderSyncController.default().directoryURLs = urls
        let joined = urls.map { $0.path }.joined(separator: ",")
        os_log("setDirectoryURLs count=%d paths=%{public}@",
               log: gsbLog, type: .default, urls.count, joined)
    }

    // MARK: - Badge Identifiers

    override func requestBadgeIdentifier(for url: URL) {
        let path = url.path

        if statusReader.isOrgFolder(path: path) {
            FIFinderSyncController.default().setBadgeIdentifier(
                GitSameBadgeConstants.BadgeID.org, for: url
            )
            return
        }

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
        let status = statusReader.currentStatus
        let timestamp = status?.timestamp

        if let repoStatus = statusReader.repoStatus(forPath: path) {
            let workspaceInfo = repoStatus.workspace.flatMap { name in
                status?.workspaces.first { $0.name == name }
            }
            return ContextMenuBuilder.build(
                for: repoStatus,
                workspaceInfo: workspaceInfo,
                timestamp: timestamp,
                socketClient: socketClient
            )
        }

        if let orgFolder = statusReader.orgFolder(forPath: path) {
            let orgRepos = (status?.repos ?? []).filter {
                $0.org == orgFolder.org && $0.workspace == orgFolder.workspace
            }
            let workspaceInfo = status?.workspaces.first { $0.name == orgFolder.workspace }
            return ContextMenuBuilder.build(
                for: orgFolder,
                repos: orgRepos,
                workspaceInfo: workspaceInfo,
                timestamp: timestamp
            )
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
