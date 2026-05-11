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
            symbolBadge(symbol: "r.square.fill", color: .systemGreen),
            label: "Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.green
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.square.fill", color: .systemBlue),
            label: "Has Local Config",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.blue
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.square.fill", color: .systemOrange),
            label: "Partially Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.orange
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.square.fill", color: .systemRed),
            label: "Uncommitted Changes",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.red
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "r.square.fill", color: .systemGray),
            label: "Git Repository",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.gray
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "o.square.fill", color: .systemBlue),
            label: "Organization",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.org
        )
        controller.setBadgeImage(
            symbolBadge(symbol: "u.square.fill", color: .systemBlue),
            label: "User",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.user
        )
    }

    /// Two-color SF Symbol badge: white letter on a colored rounded square.
    ///
    /// SF Symbols are system-rendered images that don't depend on any
    /// per-process drawing context. They sidestep the macOS 26.4
    /// FinderSync regression where both `lockFocus` and
    /// `NSImage(size:flipped:drawingHandler:)` produce blank pixel data
    /// inside the extension sandbox.
    ///
    /// `r.square.fill` / `o.square.fill` / `u.square.fill` are layered SF
    /// Symbols: layer 0 is the letter glyph, layer 1 is the filled square.
    /// Passing two palette colors forces white on the letter and the badge
    /// color on the square, restoring the visible R/O/U identity that the
    /// original lockFocus-drawn badges had. The square color is darkened
    /// 20% so the letter contrast holds at small icon sizes.
    private static func symbolBadge(symbol: String, color: NSColor) -> NSImage {
        let darker = color.shadow(withLevel: 0.2) ?? color
        let config = NSImage.SymbolConfiguration(pointSize: 256, weight: .heavy)
            .applying(NSImage.SymbolConfiguration(paletteColors: [.white, darker]))
        let image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
            .withSymbolConfiguration(config)
        return image ?? NSImage(size: NSSize(width: 16, height: 16))
    }

}
