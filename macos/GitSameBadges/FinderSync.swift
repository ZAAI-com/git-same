// FinderSync.swift
// macOS FinderSync extension that displays git status badges and context menus.

import Cocoa
import FinderSync
import os

private let gsbLog = OSLog(subsystem: "com.zaai.git-same.badges", category: "ext")

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
            self?.prefillBadges()
        }
        statusReader.startWatching()

        // StatusReader reads the file eagerly in its init but only invokes
        // onStatusUpdate on subsequent file-change events. Without this call,
        // directoryURLs stays empty until the monitor next rewrites the file,
        // so Finder never asks us for badges.
        updateMonitoredDirectories()
        prefillBadges()
    }

    // MARK: - Monitored Directories

    private func updateMonitoredDirectories() {
        guard let status = statusReader.currentStatus else {
            os_log("updateMonitoredDirectories: no status yet", log: gsbLog, type: .default)
            return
        }

        var urls = Set<URL>()
        // Prefer the monitor-provided monitored_roots (workspace roots ∪ ambient
        // scan roots). Fall back to the workspace+custom_folders union for
        // older monitors that predate that field.
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

        // Seed Finder's badge cache now that directoryURLs includes these
        // roots — any URL in status.json under them will get its real badge
        // before Finder's first paint request.
        prefillBadges()
    }

    // MARK: - Badge Identifiers

    override func requestBadgeIdentifier(for url: URL) {
        let path = url.path
        let resolved = url.resolvingSymlinksInPath().path
        let controller = FIFinderSyncController.default()

        if let orgFolder = orgFolderLookup(path: path, resolved: resolved) {
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgesConstants.BadgeID.user
                : GitSameBadgesConstants.BadgeID.org
            controller.setBadgeIdentifier(finalID, for: url)
            return
        }

        if let repoStatus = repoLookup(path: path, resolved: resolved) {
            controller.setBadgeIdentifier(badgeID(for: repoStatus.badge), for: url)
            return
        }

        // Unknown path under a monitored root: no badge. Nudge the monitor so
        // its next ambient scan picks up any new repo here; prefillBadges
        // then paints the real (or grey-ambient) badge on reload.
        requestRefresh(path: resolved)
    }

    /// Look up a repo status under both the raw URL path and the symlink-
    /// resolved path. Needed because Finder may present folders reached
    /// through volume aliases (e.g. /Volumes/Manuel-SSD-4TB -> /) with the
    /// alias prefix, while the monitor writes canonical paths to status.json.
    private func repoLookup(path: String, resolved: String) -> FinderRepoStatus? {
        if let hit = statusReader.repoStatus(forPath: path) { return hit }
        if resolved != path, let hit = statusReader.repoStatus(forPath: resolved) {
            return hit
        }
        return nil
    }

    private func orgFolderLookup(path: String, resolved: String) -> OrgFolderInfo? {
        if let hit = statusReader.orgFolder(forPath: path) { return hit }
        if resolved != path, let hit = statusReader.orgFolder(forPath: resolved) {
            return hit
        }
        return nil
    }

    private func badgeID(for badge: Badge) -> String {
        switch badge {
        case .green: return GitSameBadgesConstants.BadgeID.green
        case .blue: return GitSameBadgesConstants.BadgeID.blue
        case .orange: return GitSameBadgesConstants.BadgeID.orange
        case .red: return GitSameBadgesConstants.BadgeID.red
        case .gray: return GitSameBadgesConstants.BadgeID.gray
        }
    }

    /// Push the final badge for every known repo and org/user folder into
    /// Finder's badge cache. Called on cold start, on every status.json
    /// reload, and whenever directoryURLs changes. Idempotent: duplicate
    /// writes are free per Apple's docs ("if the identifier matches the badge
    /// in use, Finder takes no action"), so we can call this liberally.
    private func prefillBadges() {
        guard let status = statusReader.currentStatus else { return }
        let controller = FIFinderSyncController.default()

        for orgFolder in status.orgFolders ?? [] {
            let url = URL(fileURLWithPath: orgFolder.path)
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgesConstants.BadgeID.user
                : GitSameBadgesConstants.BadgeID.org
            controller.setBadgeIdentifier(finalID, for: url)
        }

        for repo in status.repos {
            let url = URL(fileURLWithPath: repo.path)
            controller.setBadgeIdentifier(badgeID(for: repo.badge), for: url)
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
        return "Git-Same"
    }

    override var toolbarItemToolTip: String {
        return "Git-Same repository status"
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
        let resolved = targetURL.resolvingSymlinksInPath().path
        let status = statusReader.currentStatus
        let timestamp = status?.timestamp

        if let repoStatus = repoLookup(path: path, resolved: resolved) {
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

        if let orgFolder = orgFolderLookup(path: path, resolved: resolved) {
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
