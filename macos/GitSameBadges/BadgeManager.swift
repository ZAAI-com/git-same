// BadgeManager.swift
// Registers badge images with the FinderSync controller.

import Cocoa
import FinderSync

enum BadgeManager {
    /// Register all badge images with FinderSync.
    /// Called once during extension initialization.
    static func registerBadges() {
        let controller = FIFinderSyncController.default()

        controller.setBadgeImage(
            symbolBadge(symbol: "r.circle.fill", color: .systemGreen),
            label: "Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.green
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.circle.fill", color: .systemBlue),
            label: "Has Local Config",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.blue
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.circle.fill", color: .systemOrange),
            label: "Partially Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.orange
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.circle.fill", color: .systemRed),
            label: "Uncommitted Changes",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.red
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.circle.fill", color: .systemGray),
            label: "Git Repository",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.gray
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "o.circle.fill", color: .systemPurple),
            label: "Organization",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.org
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "u.circle.fill", color: .systemTeal),
            label: "User",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.user
        )
    }

    /// Colored SF Symbol badge.
    ///
    /// SF Symbols are system-rendered images that don't depend on any
    /// per-process drawing context. They sidestep the macOS 26.4
    /// FinderSync regression where both `lockFocus` and
    /// `NSImage(size:flipped:drawingHandler:)` produce blank pixel data
    /// inside the extension sandbox.
    private static func symbolBadge(symbol: String, color: NSColor) -> NSImage {
        let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .heavy)
            .applying(NSImage.SymbolConfiguration(paletteColors: [color]))
        let image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
            .withSymbolConfiguration(config)
        return image ?? NSImage(size: NSSize(width: 16, height: 16))
    }

}
