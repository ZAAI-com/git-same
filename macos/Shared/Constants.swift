// Constants.swift
// Shared constants between the host app and FinderSync extension.

import Foundation

enum GitSameBadgeConstants {
    /// Real $HOME, bypassing the sandbox container redirect that
    /// FileManager.default.homeDirectoryForCurrentUser applies.
    static var realHomeDirectory: String {
        if let pw = getpwuid(getuid()), let home = pw.pointee.pw_dir {
            return String(cString: home)
        }
        return NSHomeDirectory()
    }

    /// Path to the status JSON file.
    static var statusFilePath: String {
        return "\(realHomeDirectory)/.config/git-same/finder/status.json"
    }

    /// Path to the Unix socket for refresh requests.
    static var socketPath: String {
        return "\(realHomeDirectory)/.config/git-same/finder/finder.sock"
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
        static let gray = "git-gray"
        static let org = "org"
        static let user = "user"
    }
}
