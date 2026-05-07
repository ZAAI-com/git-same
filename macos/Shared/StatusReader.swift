// StatusReader.swift
// Watches the daemon's status.json file and parses it.

import Foundation

/// Reads and watches the Finder status JSON file.
class StatusReader {
    static let shared = StatusReader()

    /// Callback invoked when the status file changes.
    var onStatusUpdate: (() -> Void)?

    /// The current parsed status.
    private(set) var currentStatus: FinderStatus?

    /// Lookup cache for repo status by path.
    private var reposByPath: [String: FinderRepoStatus] = [:]

    /// Lookup cache for org folders by path.
    private var orgFoldersByPath: [String: OrgFolderInfo] = [:]

    private var fileMonitor: DispatchSourceFileSystemObject?
    private var fileDescriptor: Int32 = -1

    private init() {
        reload()
    }

    /// Start watching the status file for changes.
    func startWatching() {
        let path = GitSameBadgesConstants.statusFilePath

        // Open file descriptor for monitoring
        fileDescriptor = open(path, O_EVTONLY)
        guard fileDescriptor >= 0 else {
            // File doesn't exist yet — try again periodically
            DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + 5) { [weak self] in
                self?.startWatching()
            }
            return
        }

        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fileDescriptor,
            eventMask: [.write, .rename, .delete],
            queue: DispatchQueue.global(qos: .utility)
        )

        source.setEventHandler { [weak self] in
            self?.reload()
            DispatchQueue.main.async {
                self?.onStatusUpdate?()
            }
        }

        source.setCancelHandler { [weak self] in
            if let fd = self?.fileDescriptor, fd >= 0 {
                close(fd)
                self?.fileDescriptor = -1
            }
        }

        fileMonitor = source
        source.resume()

        // The DispatchSource only fires on subsequent writes. If the file
        // already exists when we first successfully open it (e.g. the
        // extension started before the daemon and this is a retry that
        // finally caught the file), seed currentStatus now so observers see
        // the status without waiting for the next daemon write.
        reload()
        DispatchQueue.main.async { [weak self] in
            self?.onStatusUpdate?()
        }
    }

    /// Stop watching the status file.
    func stopWatching() {
        fileMonitor?.cancel()
        fileMonitor = nil
    }

    /// Reload and parse the status file.
    func reload() {
        let path = GitSameBadgesConstants.statusFilePath
        guard let data = FileManager.default.contents(atPath: path) else { return }

        do {
            let decoder = JSONDecoder()
            let status = try decoder.decode(FinderStatus.self, from: data)

            // Update lookup caches
            var repoMap: [String: FinderRepoStatus] = [:]
            for repo in status.repos {
                repoMap[repo.path] = repo
            }

            var orgMap: [String: OrgFolderInfo] = [:]
            for org in status.orgFolders ?? [] {
                orgMap[org.path] = org
            }

            self.currentStatus = status
            self.reposByPath = repoMap
            self.orgFoldersByPath = orgMap
        } catch {
            // Ignore parse errors (file might be mid-write, though atomic rename should prevent this)
        }
    }

    /// Get the status for a repo at the given path.
    func repoStatus(forPath path: String) -> FinderRepoStatus? {
        return reposByPath[path]
    }

    /// Check if the given path is an org folder.
    func isOrgFolder(path: String) -> Bool {
        return orgFoldersByPath[path] != nil
    }

    /// Get the org-folder info for the given path, if any.
    func orgFolder(forPath path: String) -> OrgFolderInfo? {
        return orgFoldersByPath[path]
    }

    deinit {
        stopWatching()
    }
}
