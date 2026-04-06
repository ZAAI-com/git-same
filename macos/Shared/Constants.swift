// Constants.swift
// Shared constants between the host app and FinderSync extension.

import Foundation

enum GitSameBadgeConstants {
    /// Path to the status JSON file.
    static var statusFilePath: String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        return "\(home)/.config/git-same/finder/status.json"
    }

    /// Path to the Unix socket for refresh requests.
    static var socketPath: String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        return "\(home)/.config/git-same/finder/finder.sock"
    }

    /// Path to the git-same binary.
    static var daemonBinaryPath: String {
        // Check common installation locations
        let candidates = [
            "/usr/local/bin/git-same",
            "/opt/homebrew/bin/git-same",
            "/usr/bin/git-same",
        ]
        for path in candidates {
            if FileManager.default.isExecutableFile(atPath: path) {
                return path
            }
        }
        return "git-same" // Fall back to PATH lookup
    }

    /// Badge identifiers used by FinderSync.
    enum BadgeID {
        static let green = "git-green"
        static let blue = "git-blue"
        static let orange = "git-orange"
        static let red = "git-red"
        static let org = "org"
    }
}
