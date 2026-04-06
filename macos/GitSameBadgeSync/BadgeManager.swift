// BadgeManager.swift
// Registers badge images with the FinderSync controller.

import Cocoa
import FinderSync

enum BadgeManager {
    /// Register all badge images with FinderSync.
    /// Called once during extension initialization.
    static func registerBadges() {
        let controller = FIFinderSyncController.default()

        // Green badge: fully synced, safe to delete
        if let greenImage = createBadgeImage(color: .systemGreen) {
            controller.setBadgeImage(greenImage, label: "Synced", forBadgeIdentifier: GitSameBadgeConstants.BadgeID.green)
        }

        // Blue badge: synced but has important ignored files
        if let blueImage = createBadgeImage(color: .systemBlue) {
            controller.setBadgeImage(blueImage, label: "Has Local Config", forBadgeIdentifier: GitSameBadgeConstants.BadgeID.blue)
        }

        // Orange badge: main synced, worktrees/branches diverge
        if let orangeImage = createBadgeImage(color: .systemOrange) {
            controller.setBadgeImage(orangeImage, label: "Partially Synced", forBadgeIdentifier: GitSameBadgeConstants.BadgeID.orange)
        }

        // Red badge: uncommitted changes or unpushed commits
        if let redImage = createBadgeImage(color: .systemRed) {
            controller.setBadgeImage(redImage, label: "Uncommitted Changes", forBadgeIdentifier: GitSameBadgeConstants.BadgeID.red)
        }

        // Org folder badge
        if let orgImage = createOrgBadgeImage() {
            controller.setBadgeImage(orgImage, label: "Organization", forBadgeIdentifier: GitSameBadgeConstants.BadgeID.org)
        }
    }

    /// Create a colored dot badge image.
    private static func createBadgeImage(color: NSColor) -> NSImage? {
        let size = NSSize(width: 16, height: 16)
        let image = NSImage(size: size)
        image.lockFocus()

        // Draw a filled circle
        let rect = NSRect(x: 2, y: 2, width: 12, height: 12)
        let path = NSBezierPath(ovalIn: rect)
        color.setFill()
        path.fill()

        // Draw a thin border
        NSColor.white.withAlphaComponent(0.5).setStroke()
        path.lineWidth = 1.0
        path.stroke()

        image.unlockFocus()
        return image
    }

    /// Create an org folder badge image.
    private static func createOrgBadgeImage() -> NSImage? {
        let size = NSSize(width: 16, height: 16)
        let image = NSImage(size: size)
        image.lockFocus()

        // Draw a building/org icon using a simple shape
        let rect = NSRect(x: 3, y: 2, width: 10, height: 12)
        let path = NSBezierPath(roundedRect: rect, xRadius: 1, yRadius: 1)
        NSColor.systemPurple.setFill()
        path.fill()

        // Draw windows
        NSColor.white.setFill()
        NSBezierPath(rect: NSRect(x: 5, y: 9, width: 2, height: 2)).fill()
        NSBezierPath(rect: NSRect(x: 9, y: 9, width: 2, height: 2)).fill()
        NSBezierPath(rect: NSRect(x: 5, y: 5, width: 2, height: 2)).fill()
        NSBezierPath(rect: NSRect(x: 9, y: 5, width: 2, height: 2)).fill()

        image.unlockFocus()
        return image
    }
}
