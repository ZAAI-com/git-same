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

    /// URLs whose final colored badge has already been pushed to Finder. Used
    /// to skip the grey-R flash on subsequent folder switches so it only
    /// appears once per URL per extension lifetime.
    private var coloredURLs: Set<URL> = []

    /// How long to leave the grey "R" on screen before swapping in the real
    /// color. Needs to be large enough that Finder's render pipeline actually
    /// paints the grey frame; DispatchQueue.main.async alone is too fast and
    /// Finder coalesces it with the color update.
    private static let greyHoldDuration: TimeInterval = 0.25

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
        let resolved = url.resolvingSymlinksInPath().path

        if let orgFolder = orgFolderLookup(path: path, resolved: resolved) {
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgeConstants.BadgeID.user
                : GitSameBadgeConstants.BadgeID.org
            applyBadge(finalID: finalID, for: url)
            return
        }

        if let repoStatus = repoLookup(path: path, resolved: resolved) {
            applyBadge(finalID: badgeID(for: repoStatus.badge), for: url)
            return
        }

        // Unknown path inside a monitored directory: nudge the daemon so the
        // real color arrives on its next scan instead of waiting up to 30s.
        requestRefresh(path: resolved)
    }

    /// Look up a repo status under both the raw URL path and the symlink-
    /// resolved path. Needed because Finder may present folders reached
    /// through volume aliases (e.g. /Volumes/Manuel-SSD-4TB -> /) with the
    /// alias prefix, while the daemon writes canonical paths to status.json.
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

    /// On first sight of a repo URL, paint grey "R" immediately and schedule
    /// the real color after a short hold so Finder actually renders the grey
    /// frame. Org and user folders render their own letter (O / U) and skip
    /// the placeholder entirely — flashing grey-R before purple-O would be a
    /// letter swap, not a loading hint. On subsequent calls for a URL we've
    /// already painted, set the final badge directly.
    private func applyBadge(finalID: String, for url: URL) {
        let controller = FIFinderSyncController.default()

        let isRBadge = finalID == GitSameBadgeConstants.BadgeID.green
            || finalID == GitSameBadgeConstants.BadgeID.blue
            || finalID == GitSameBadgeConstants.BadgeID.orange
            || finalID == GitSameBadgeConstants.BadgeID.red

        if !isRBadge {
            // gray, org, user — set directly, no placeholder.
            controller.setBadgeIdentifier(finalID, for: url)
            if finalID == GitSameBadgeConstants.BadgeID.gray {
                coloredURLs.remove(url)
            } else {
                coloredURLs.insert(url)
            }
            return
        }

        if coloredURLs.contains(url) {
            controller.setBadgeIdentifier(finalID, for: url)
            return
        }

        controller.setBadgeIdentifier(GitSameBadgeConstants.BadgeID.gray, for: url)
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.greyHoldDuration) { [weak self] in
            controller.setBadgeIdentifier(finalID, for: url)
            self?.coloredURLs.insert(url)
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

    /// Pre-register grey "R" for every known repo and org-folder URL so that
    /// Finder has something to paint before it ever calls
    /// `requestBadgeIdentifier`. The grey→color flip is driven by
    /// `requestBadgeIdentifier` itself (via `applyBadge`), which is the only
    /// place guaranteed to happen at actual paint time. Scheduling the color
    /// follow-up here instead made the grey flash happen during extension
    /// startup (invisible), so this function deliberately avoids that.
    /// Already-colored URLs are refreshed to the current color without
    /// regressing to grey.
    private func prewarmBadges() {
        guard let status = statusReader.currentStatus else { return }
        let controller = FIFinderSyncController.default()

        for orgFolder in status.orgFolders ?? [] {
            let url = URL(fileURLWithPath: orgFolder.path)
            let finalID = orgFolder.ownerType == .user
                ? GitSameBadgeConstants.BadgeID.user
                : GitSameBadgeConstants.BadgeID.org
            // Org/user badges have their own letter; don't pre-set grey R for
            // them, just register the final badge directly.
            controller.setBadgeIdentifier(finalID, for: url)
            coloredURLs.insert(url)
        }

        for repo in status.repos {
            let url = URL(fileURLWithPath: repo.path)
            let finalID = badgeID(for: repo.badge)
            if coloredURLs.contains(url) || finalID == GitSameBadgeConstants.BadgeID.gray {
                controller.setBadgeIdentifier(finalID, for: url)
            } else {
                controller.setBadgeIdentifier(GitSameBadgeConstants.BadgeID.gray, for: url)
            }
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
