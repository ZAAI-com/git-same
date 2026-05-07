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
            labeledBadge(text: "R", color: .systemGreen),
            label: "Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.green
        )
        controller.setBadgeImage(
            labeledBadge(text: "R", color: .systemBlue),
            label: "Has Local Config",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.blue
        )
        controller.setBadgeImage(
            labeledBadge(text: "R", color: .systemOrange),
            label: "Partially Synced",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.orange
        )
        controller.setBadgeImage(
            labeledBadge(text: "R", color: .systemRed),
            label: "Uncommitted Changes",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.red
        )
        controller.setBadgeImage(
            labeledBadge(text: "R", color: .systemGray),
            label: "Git Repository",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.gray
        )
        controller.setBadgeImage(
            labeledBadge(text: "O", color: .systemPurple),
            label: "Organization",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.org
        )
        controller.setBadgeImage(
            labeledBadge(text: "U", color: .systemTeal),
            label: "User",
            forBadgeIdentifier: GitSameBadgesConstants.BadgeID.user
        )
    }

    /// Rounded-rect badge with centered white text on a colored fill.
    ///
    /// Drawn via NSImage(size:flipped:drawingHandler:) so the image has
    /// valid CGImage-backed pixel data in a sandboxed extension where
    /// lockFocus produces zero-pixel layers.
    private static func labeledBadge(text: String, color: NSColor) -> NSImage {
        let size = NSSize(width: 64, height: 64)
        return NSImage(size: size, flipped: false) { rect in
            let inset: CGFloat = 4
            let bodyRect = rect.insetBy(dx: inset, dy: inset)
            let body = NSBezierPath(roundedRect: bodyRect, xRadius: 14, yRadius: 14)
            color.setFill()
            body.fill()
            NSColor.white.withAlphaComponent(0.35).setStroke()
            body.lineWidth = 2.0
            body.stroke()

            let attrs: [NSAttributedString.Key: Any] = [
                .font: NSFont.systemFont(ofSize: 44, weight: .heavy),
                .foregroundColor: NSColor.white,
            ]
            let attributed = NSAttributedString(string: text, attributes: attrs)
            let textSize = attributed.size()
            let origin = NSPoint(
                x: rect.midX - textSize.width / 2,
                y: rect.midY - textSize.height / 2
            )
            attributed.draw(at: origin)
            return true
        }
    }
}
