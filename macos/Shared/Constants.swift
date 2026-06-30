// Constants.swift
// Shared constants between the host app and Badges (FinderSync) extension.
//
// IPC paths:
//   - Production: resolved through `containerURL(forSecurityApplicationGroupIdentifier:)`
//     using `appGroupIdentifier`, giving `~/Library/Group Containers/group.<TEAM>.com.zaai.git-same/`.
//   - Fallback (unsigned dev builds, formula installs): the legacy
//     `~/.config/git-same/finder/` path. This branch is only taken when the
//     app group container URL is nil, which happens when the running binary
//     does not declare `com.apple.security.application-groups` (e.g.
//     `tauri dev` or a `cargo run` of the monitor without the bundled
//     entitlements).

import Foundation

enum GitSameBadgesConstants {
    /// App group shared between the host app, the Badges extension, and the
    /// monitor. Apple requires the team-id prefix.
    /// Mirrors the Rust `git_same_core::ipc::APP_GROUP_ID`.
    static let appGroupIdentifier = "group.57KL6Y7V32.com.zaai.git-same"

    /// Real $HOME, bypassing the sandbox container redirect that
    /// FileManager.default.homeDirectoryForCurrentUser applies. Used only by
    /// the legacy fallback paths below.
    private static var realHomeDirectory: String {
        if let pw = getpwuid(getuid()), let home = pw.pointee.pw_dir {
            return String(cString: home)
        }
        return NSHomeDirectory()
    }

    /// Directory containing IPC files (status.json, finder.sock).
    ///
    /// Returns the app group container directory in production. Falls back to
    /// `~/.config/git-same/finder/` when the container URL is unavailable
    /// (unsigned dev builds, or non-cask installs where the entitlement is
    /// not present).
    static var ipcDirectory: String {
        if let url = FileManager.default.containerURL(
            forSecurityApplicationGroupIdentifier: appGroupIdentifier
        ) {
            return url.path
        }
        return "\(realHomeDirectory)/.config/git-same/finder"
    }

    /// Path to the status JSON file.
    static var statusFilePath: String {
        return "\(ipcDirectory)/status.json"
    }

    /// Path to the Unix socket for refresh requests.
    static var socketPath: String {
        return "\(ipcDirectory)/finder.sock"
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
