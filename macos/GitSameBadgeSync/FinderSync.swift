// FinderSync.swift
// macOS FinderSync extension that displays git status badges and context menus.

import Cocoa
import FinderSync
import os

private let gsbLog = OSLog(subsystem: "com.zaai.git-same.GitSameBadge.FinderSync", category: "ext")

class FinderSync: FIFinderSync {

    let statusReader = StatusReader.shared
    let socketClient = SocketClient()

    private var lastRefreshRequest: [String: Date] = [:]
    private static let refreshThrottle: TimeInterval = 10.0

    override init() {
        super.init()
        os_log("FinderSync init entered", log: gsbLog, type: .default)

        BadgeManager.registerBadges()

        statusReader.onStatusUpdate = { [weak self] in
            self?.updateMonitoredDirectories()
            self?.prewarmBadges()
        }
        statusReader.startWatching()

        // StatusReader reads the file eagerly in its init but only invokes
        // onStatusUpdate on subsequent file-change events. Without this call,
        // directoryURLs stays empty until the daemon next rewrites the file,
        // so Finder never asks us for badges.
        updateMonitoredDirectories()
        prewarmBadges()
    }

    // MARK: - Monitored Directories

    private func updateMonitoredDirectories() {
        guard let status = statusReader.currentStatus else {
            os_log("updateMonitoredDirectories: no status yet", log: gsbLog, type: .default)
            return
        }

        var urls = Set<URL>()
        // Prefer the daemon-provided monitored_roots (workspace roots ∪ ambient
        // scan roots). Fall back to the workspace+custom_folders union for
        // older daemons that predate that field.
        if let roots = status.monitoredRoots, !roots.isEmpty {
            for root in roots {
                urls.insert(URL(fileURLWithPath: root))
            }
        } else {
            for workspace in status.workspaces {
                urls.insert(URL(fileURLWithPath: workspace.root))
            }
            for folder in status.customFolders ?? [] {
                urls.insert(URL(fileURLWithPath: folder))
            }
        }

        FIFinderSyncController.default().directoryURLs = urls
        let joined = urls.map { $0.path }.joined(separator: ",")
        os_log("setDirectoryURLs count=%d paths=%{public}@",
               log: gsbLog, type: .default, urls.count, joined)
    }

    // MARK: - Badge Identifiers

    override func requestBadgeIdentifier(for url: URL) {
        let path = url.path

        if let orgFolder = statusReader.orgFolder(forPath: path) {
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgeConstants.BadgeID.user
                : GitSameBadgeConstants.BadgeID.org
            applyBadge(finalID: finalID, for: url)
            return
        }

        if let repoStatus = statusReader.repoStatus(forPath: path) {
            applyBadge(finalID: badgeID(for: repoStatus.badge), for: url)
            return
        }

        // Unknown path inside a monitored directory: nudge the daemon so the
        // real color arrives on its next scan instead of waiting up to 30s.
        requestRefresh(path: path)
    }

    /// Render grey "R" synchronously so Finder has something to draw right
    /// away, then swap in the real color on the next runloop tick. When the
    /// final badge is already grey (daemon-marked ambient repo), skip the
    /// second call.
    private func applyBadge(finalID: String, for url: URL) {
        let controller = FIFinderSyncController.default()
        if finalID == GitSameBadgeConstants.BadgeID.gray {
            controller.setBadgeIdentifier(finalID, for: url)
            return
        }
        controller.setBadgeIdentifier(GitSameBadgeConstants.BadgeID.gray, for: url)
        DispatchQueue.main.async {
            controller.setBadgeIdentifier(finalID, for: url)
        }
    }

    private func badgeID(for badge: Badge) -> String {
        switch badge {
        case .green: return GitSameBadgeConstants.BadgeID.green
        case .blue: return GitSameBadgeConstants.BadgeID.blue
        case .orange: return GitSameBadgeConstants.BadgeID.orange
        case .red: return GitSameBadgeConstants.BadgeID.red
        case .gray: return GitSameBadgeConstants.BadgeID.gray
        }
    }

    /// Pre-register badges for every known repo and org-folder URL. Finder's
    /// first-paint-per-URL pipeline dominates the visible latency, so setting
    /// badges before Finder asks lets the UI render them without a blank gap.
    private func prewarmBadges() {
        guard let status = statusReader.currentStatus else { return }

        for orgFolder in status.orgFolders ?? [] {
            let url = URL(fileURLWithPath: orgFolder.path)
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgeConstants.BadgeID.user
                : GitSameBadgeConstants.BadgeID.org
            applyBadge(finalID: finalID, for: url)
        }

        for repo in status.repos {
            let url = URL(fileURLWithPath: repo.path)
            applyBadge(finalID: badgeID(for: repo.badge), for: url)
        }
    }

    private func requestRefresh(path: String) {
        let now = Date()
        if let last = lastRefreshRequest[path],
           now.timeIntervalSince(last) < Self.refreshThrottle
        {
            return
        }
        lastRefreshRequest[path] = now
        socketClient.send("REFRESH \(path)") { _ in }
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
