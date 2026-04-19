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
            dotImage(color: .systemGreen),
            label: "Synced",
            forBadgeIdentifier: GitSameBadgeConstants.BadgeID.green
        )
        controller.setBadgeImage(
            dotImage(color: .systemBlue),
            label: "Has Local Config",
            forBadgeIdentifier: GitSameBadgeConstants.BadgeID.blue
        )
        controller.setBadgeImage(
            dotImage(color: .systemOrange),
            label: "Partially Synced",
            forBadgeIdentifier: GitSameBadgeConstants.BadgeID.orange
        )
        controller.setBadgeImage(
            dotImage(color: .systemRed),
            label: "Uncommitted Changes",
            forBadgeIdentifier: GitSameBadgeConstants.BadgeID.red
        )
        controller.setBadgeImage(
            orgImage(),
            label: "Organization",
            forBadgeIdentifier: GitSameBadgeConstants.BadgeID.org
        )
    }

    /// Colored dot badge, drawn via closure so it works without a
    /// display context (lockFocus produces zero-pixel layer data in
    /// sandboxed extensions).
    private static func dotImage(color: NSColor) -> NSImage {
        let size = NSSize(width: 16, height: 16)
        return NSImage(size: size, flipped: false) { rect in
            let circle = NSBezierPath(ovalIn: NSRect(x: 2, y: 2, width: 12, height: 12))
            color.setFill()
            circle.fill()
            NSColor.white.withAlphaComponent(0.5).setStroke()
            circle.lineWidth = 1.0
            circle.stroke()
            return true
        }
    }

    /// Org-folder badge: purple building silhouette with four windows.
    private static func orgImage() -> NSImage {
        let size = NSSize(width: 16, height: 16)
        return NSImage(size: size, flipped: false) { rect in
            let body = NSBezierPath(
                roundedRect: NSRect(x: 3, y: 2, width: 10, height: 12),
                xRadius: 1,
                yRadius: 1
            )
            NSColor.systemPurple.setFill()
            body.fill()

            NSColor.white.setFill()
            for (x, y) in [(5, 9), (9, 9), (5, 5), (9, 5)] {
                NSBezierPath(rect: NSRect(x: x, y: y, width: 2, height: 2)).fill()
            }
            return true
        }
    }
}
