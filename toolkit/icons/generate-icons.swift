#!/usr/bin/env swift
// Generates Git-Same macOS app icon concept variants as 1024x1024 PNGs.
//
// Palette uses the macOS system colors that the Finder Badges register in
// macos/GitSameBadges/BadgeManager.swift (.systemBlue, .systemGreen, etc.),
// resolved to their sRGB light-mode values so PNGs stay deterministic.
//
// Usage: swift toolkit/icons/generate-icons.swift [--variant <name>] [--out <dir>] [--size <px>]
//   <name> is one of: twin, sync, folder-pair, folder-icon, wordmark, tui-banner, all (default)

import AppKit
import CoreGraphics
import Foundation

// MARK: - Palette

private func sRGB(_ c: NSColor) -> NSColor {
    c.usingColorSpace(.sRGB) ?? c
}

struct Palette {
    static let blue   = sRGB(.systemBlue)    // Finder badge: Has Local Config / Org / User
    static let green  = sRGB(.systemGreen)   // Finder badge: Synced
    static let orange = sRGB(.systemOrange)  // Finder badge: Partially Synced
    static let red    = sRGB(.systemRed)     // Finder badge: Uncommitted Changes
    static let gray   = sRGB(.systemGray)    // Finder badge: Git Repository
    static let cream  = NSColor(srgbRed: 0xF5/255.0, green: 0xF1/255.0, blue: 0xE8/255.0, alpha: 1)
    static let ink    = NSColor(srgbRed: 0x0B/255.0, green: 0x1B/255.0, blue: 0x2A/255.0, alpha: 1)
}

let allVariants = ["twin", "sync", "folder-pair", "folder-icon", "wordmark", "tui-banner"]

// MARK: - Args

struct Args {
    var variant: String = "all"
    var out: String = "crates/git-same-app/icons/variants"
    var size: Int = 1024
}

func parseArgs() -> Args {
    var a = Args()
    var it = CommandLine.arguments.dropFirst().makeIterator()
    while let arg = it.next() {
        switch arg {
        case "--variant", "-v":
            if let v = it.next() { a.variant = v }
        case "--out", "-o":
            if let v = it.next() { a.out = v }
        case "--size", "-s":
            if let v = it.next(), let n = Int(v) { a.size = n }
        case "-h", "--help":
            FileHandle.standardError.write(Data("""
                generate-icons.swift [--variant <name>] [--out <dir>] [--size <px>]
                  variants: \(allVariants.joined(separator: ", ")), all
                """.utf8))
            exit(0)
        default:
            FileHandle.standardError.write(Data("unknown arg: \(arg)\n".utf8))
            exit(2)
        }
    }
    return a
}

// MARK: - Rendering helpers

func makeContext(size: Int) -> CGContext {
    let cs = CGColorSpaceCreateDeviceRGB()
    let ctx = CGContext(data: nil,
                        width: size, height: size,
                        bitsPerComponent: 8,
                        bytesPerRow: size * 4,
                        space: cs,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .high
    ctx.setShouldAntialias(true)
    return ctx
}

func savePNG(_ ctx: CGContext, to path: String) throws {
    guard let img = ctx.makeImage() else {
        throw NSError(domain: "icon", code: 1, userInfo: [NSLocalizedDescriptionKey: "ctx.makeImage failed"])
    }
    let rep = NSBitmapImageRep(cgImage: img)
    guard let data = rep.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "icon", code: 2, userInfo: [NSLocalizedDescriptionKey: "PNG encode failed"])
    }
    let url = URL(fileURLWithPath: path)
    try FileManager.default.createDirectory(at: url.deletingLastPathComponent(),
                                            withIntermediateDirectories: true)
    try data.write(to: url)
}

// Continuous-curvature squircle approximation: rounded rect with r ≈ 0.2237 * side.
// Good enough at 1024px to read as a macOS app icon.
func squirclePath(in rect: CGRect) -> CGPath {
    let r = min(rect.width, rect.height) * 0.2237
    return CGPath(roundedRect: rect, cornerWidth: r, cornerHeight: r, transform: nil)
}

func drawGradientBackground(_ ctx: CGContext, size: CGFloat) {
    let rect = CGRect(x: 0, y: 0, width: size, height: size)
    ctx.saveGState()
    ctx.addPath(squirclePath(in: rect))
    ctx.clip()

    let cs = CGColorSpaceCreateDeviceRGB()
    let colors: [CGColor] = [Palette.blue.cgColor, Palette.green.cgColor]
    let stops: [CGFloat] = [0.0, 1.0]
    let grad = CGGradient(colorsSpace: cs, colors: colors as CFArray, locations: stops)!
    // Diagonal: top-left → bottom-right.
    ctx.drawLinearGradient(grad,
                           start: CGPoint(x: 0, y: size),
                           end: CGPoint(x: size, y: 0),
                           options: [])
    ctx.restoreGState()
}

func drawAttributedText(_ ctx: CGContext, _ text: String, attrs: [NSAttributedString.Key: Any], at point: CGPoint) {
    let attr = NSAttributedString(string: text, attributes: attrs)
    let line = CTLineCreateWithAttributedString(attr)
    ctx.textPosition = point
    CTLineDraw(line, ctx)
}

func textBounds(_ text: String, attrs: [NSAttributedString.Key: Any]) -> CGRect {
    let attr = NSAttributedString(string: text, attributes: attrs)
    let line = CTLineCreateWithAttributedString(attr)
    return CTLineGetBoundsWithOptions(line, .useOpticalBounds)
}

// MARK: - Variant 1: twin (liquid glass)

func drawTwin(_ ctx: CGContext, size: CGFloat) {
    drawGradientBackground(ctx, size: size)

    // Two overlapping rounded-square "repo tiles", rendered as Liquid Glass:
    // translucent panels with a top specular highlight, soft drop shadow,
    // hairline rim stroke, and a subtle tint hinting at the badge color
    // (green for the back/"synced" tile, blue for the front/"local config" tile).
    let tileSide = size * 0.50
    let r = tileSide * 0.26
    let cx = size / 2
    let cy = size / 2
    let off = size * 0.075

    let backRect = CGRect(x: cx - tileSide/2 - off,
                          y: cy - tileSide/2 + off,
                          width: tileSide, height: tileSide)
    let frontRect = CGRect(x: cx - tileSide/2 + off,
                           y: cy - tileSide/2 - off,
                           width: tileSide, height: tileSide)

    drawGlassTile(ctx, rect: backRect, cornerRadius: r,
                  tint: Palette.green, rowColor: Palette.green, size: size,
                  excludeRect: frontRect, excludeCornerRadius: r)
    drawGlassTile(ctx, rect: frontRect, cornerRadius: r,
                  tint: Palette.blue, rowColor: Palette.blue, size: size,
                  excludeRect: nil, excludeCornerRadius: 0)
}

/// Renders one Liquid Glass tile with three list rows inside.
/// If `excludeRect` is provided, the tile's row content is clipped to the
/// area outside that rect — used so the back tile's rows don't bleed
/// through the front tile.
func drawGlassTile(_ ctx: CGContext, rect: CGRect, cornerRadius r: CGFloat,
                   tint: NSColor, rowColor: NSColor, size: CGFloat,
                   excludeRect: CGRect?, excludeCornerRadius: CGFloat) {
    let path = CGPath(roundedRect: rect, cornerWidth: r, cornerHeight: r, transform: nil)
    let cs = CGColorSpaceCreateDeviceRGB()

    // 1. Soft drop shadow under the panel.
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -size * 0.012),
                  blur: size * 0.035,
                  color: NSColor.black.withAlphaComponent(0.28).cgColor)
    ctx.setFillColor(NSColor.black.withAlphaComponent(0.001).cgColor)
    ctx.addPath(path)
    ctx.fillPath()
    ctx.restoreGState()

    // 2. Glass body: translucent white with a hint of tint, clipped to the panel.
    ctx.saveGState()
    ctx.addPath(path)
    ctx.clip()

    // 2a. Base translucent fill.
    ctx.setFillColor(NSColor.white.withAlphaComponent(0.32).cgColor)
    ctx.fill(rect)

    // 2b. Vertical glass gradient: brighter at top, slightly cooler at bottom.
    let bodyGrad = CGGradient(colorsSpace: cs, colors: [
        NSColor.white.withAlphaComponent(0.55).cgColor,
        NSColor.white.withAlphaComponent(0.10).cgColor,
    ] as CFArray, locations: [0.0, 1.0])!
    ctx.drawLinearGradient(bodyGrad,
                           start: CGPoint(x: rect.midX, y: rect.maxY),
                           end:   CGPoint(x: rect.midX, y: rect.minY),
                           options: [])

    // 2c. Subtle tint band hugging the bottom edge, so the back tile reads
    // green and the front tile reads blue without going opaque.
    let tintGrad = CGGradient(colorsSpace: cs, colors: [
        tint.withAlphaComponent(0.0).cgColor,
        tint.withAlphaComponent(0.22).cgColor,
    ] as CFArray, locations: [0.0, 1.0])!
    ctx.drawLinearGradient(tintGrad,
                           start: CGPoint(x: rect.midX, y: rect.maxY),
                           end:   CGPoint(x: rect.midX, y: rect.minY),
                           options: [])

    // 2d. Top specular highlight: thin bright band along the upper edge.
    let specHeight = rect.height * 0.18
    let specRect = CGRect(x: rect.minX, y: rect.maxY - specHeight,
                          width: rect.width, height: specHeight)
    let specGrad = CGGradient(colorsSpace: cs, colors: [
        NSColor.white.withAlphaComponent(0.65).cgColor,
        NSColor.white.withAlphaComponent(0.0).cgColor,
    ] as CFArray, locations: [0.0, 1.0])!
    ctx.drawLinearGradient(specGrad,
                           start: CGPoint(x: specRect.midX, y: specRect.maxY),
                           end:   CGPoint(x: specRect.midX, y: specRect.minY),
                           options: [])

    // 2e. Rows on top — solid for contrast against the glass.
    // If an exclude rect is provided, clip rows to (tileRect MINUS excludeRect)
    // using even-odd fill so we don't paint rows beneath the front tile.
    if let ex = excludeRect {
        ctx.saveGState()
        let outer = CGPath(rect: rect, transform: nil)
        let inner = CGPath(roundedRect: ex,
                           cornerWidth: excludeCornerRadius,
                           cornerHeight: excludeCornerRadius,
                           transform: nil)
        let combined = CGMutablePath()
        combined.addPath(outer)
        combined.addPath(inner)
        ctx.addPath(combined)
        ctx.clip(using: .evenOdd)
        drawRepoRows(ctx, in: rect, color: rowColor)
        ctx.restoreGState()
    } else {
        drawRepoRows(ctx, in: rect, color: rowColor)
    }
    ctx.restoreGState()

    // 3. Hairline rim stroke for definition.
    ctx.saveGState()
    ctx.addPath(path)
    ctx.setStrokeColor(NSColor.white.withAlphaComponent(0.55).cgColor)
    ctx.setLineWidth(size * 0.004)
    ctx.strokePath()
    ctx.restoreGState()
}

func drawRepoRows(_ ctx: CGContext, in rect: CGRect, color: NSColor) {
    let rows = 3
    let pad = rect.width * 0.16
    let inner = rect.insetBy(dx: pad, dy: pad)
    let rowHeight = inner.height * 0.18
    let gap = (inner.height - CGFloat(rows) * rowHeight) / CGFloat(rows - 1) * 0.6
    let totalH = CGFloat(rows) * rowHeight + CGFloat(rows - 1) * gap
    let startY = inner.midY + totalH/2 - rowHeight

    ctx.saveGState()
    ctx.setFillColor(color.cgColor)
    for i in 0..<rows {
        let y = startY - CGFloat(i) * (rowHeight + gap)
        // Dot.
        let dotR = rowHeight * 0.42
        let dotX = inner.minX + dotR
        ctx.fillEllipse(in: CGRect(x: dotX - dotR, y: y + rowHeight/2 - dotR,
                                   width: dotR * 2, height: dotR * 2))
        // Bar.
        let barX = dotX + dotR * 1.6
        let barRect = CGRect(x: barX, y: y, width: inner.maxX - barX, height: rowHeight)
        let barR = rowHeight / 2
        ctx.addPath(CGPath(roundedRect: barRect, cornerWidth: barR, cornerHeight: barR, transform: nil))
        ctx.fillPath()
    }
    ctx.restoreGState()
}

// MARK: - Variant 2: sync

func drawSync(_ ctx: CGContext, size: CGFloat) {
    drawGradientBackground(ctx, size: size)

    let cx = size / 2
    let cy = size / 2
    let radius = size * 0.30
    let stroke = size * 0.075

    // Two arc segments forming a circular sync motif, each capped by an arrow head.
    // Top arc sweeps clockwise from ~160° to ~20° (going through 90°).
    // Bottom arc sweeps clockwise from ~340° to ~200° (going through 270°).
    ctx.saveGState()
    ctx.setLineCap(.round)
    ctx.setLineWidth(stroke)

    // Top arc — cream.
    ctx.setStrokeColor(Palette.cream.cgColor)
    ctx.beginPath()
    ctx.addArc(center: CGPoint(x: cx, y: cy), radius: radius,
               startAngle: deg(160), endAngle: deg(20),
               clockwise: true)
    ctx.strokePath()
    drawArrowHead(ctx, center: CGPoint(x: cx, y: cy), radius: radius,
                  angle: deg(20), tangentClockwise: true,
                  size: stroke * 1.15, color: Palette.cream)

    // Bottom arc — cream.
    ctx.beginPath()
    ctx.setStrokeColor(Palette.cream.cgColor)
    ctx.addArc(center: CGPoint(x: cx, y: cy), radius: radius,
               startAngle: deg(340), endAngle: deg(200),
               clockwise: true)
    ctx.strokePath()
    drawArrowHead(ctx, center: CGPoint(x: cx, y: cy), radius: radius,
                  angle: deg(200), tangentClockwise: true,
                  size: stroke * 1.15, color: Palette.cream)
    ctx.restoreGState()

    // Center node: a small "branch" — a vertical line with two dots, suggesting a repo.
    let nodeColor = Palette.ink
    let nodeR = size * 0.034
    let trunkX = cx
    let trunkTopY = cy + size * 0.075
    let trunkBotY = cy - size * 0.075
    ctx.saveGState()
    ctx.setStrokeColor(nodeColor.cgColor)
    ctx.setLineWidth(size * 0.022)
    ctx.setLineCap(.round)
    ctx.beginPath()
    ctx.move(to: CGPoint(x: trunkX, y: trunkTopY))
    ctx.addLine(to: CGPoint(x: trunkX, y: trunkBotY))
    ctx.strokePath()
    ctx.setFillColor(nodeColor.cgColor)
    for y in [trunkTopY, trunkBotY, cy] {
        ctx.fillEllipse(in: CGRect(x: trunkX - nodeR, y: y - nodeR, width: nodeR * 2, height: nodeR * 2))
    }
    ctx.restoreGState()
}

func deg(_ d: CGFloat) -> CGFloat { d * .pi / 180 }

func drawArrowHead(_ ctx: CGContext, center: CGPoint, radius: CGFloat,
                   angle: CGFloat, tangentClockwise: Bool, size s: CGFloat, color: NSColor) {
    let tip = CGPoint(x: center.x + radius * cos(angle), y: center.y + radius * sin(angle))
    // Tangent direction at the arc endpoint.
    let tAngle = angle + (tangentClockwise ? -.pi/2 : .pi/2)
    let back = CGPoint(x: tip.x - cos(tAngle) * s * 1.8,
                       y: tip.y - sin(tAngle) * s * 1.8)
    let perp = CGPoint(x: cos(tAngle + .pi/2) * s, y: sin(tAngle + .pi/2) * s)
    let p1 = CGPoint(x: back.x + perp.x, y: back.y + perp.y)
    let p2 = CGPoint(x: back.x - perp.x, y: back.y - perp.y)
    ctx.saveGState()
    ctx.setFillColor(color.cgColor)
    ctx.beginPath()
    ctx.move(to: tip)
    ctx.addLine(to: p1)
    ctx.addLine(to: p2)
    ctx.closePath()
    ctx.fillPath()
    ctx.restoreGState()
}

// MARK: - Variant 3: folder-pair

func drawFolderPair(_ ctx: CGContext, size: CGFloat) {
    drawGradientBackground(ctx, size: size)

    let w = size * 0.62
    let h = size * 0.50
    let cx = size / 2
    let cy = size / 2

    // Back folder, offset up-left, slightly smaller, tinted darker green.
    let backRect = CGRect(x: cx - w/2 - size * 0.06,
                          y: cy - h/2 + size * 0.08,
                          width: w * 0.92, height: h * 0.92)
    drawFolder(ctx, rect: backRect, body: Palette.green, tab: Palette.green.blended(withFraction: 0.25, of: .black) ?? Palette.green)

    // Front folder, full size, cream.
    let frontRect = CGRect(x: cx - w/2 + size * 0.04,
                           y: cy - h/2 - size * 0.04,
                           width: w, height: h)
    drawFolder(ctx, rect: frontRect, body: Palette.cream, tab: Palette.blue)

    // Small arrow from back-folder corner to front folder, suggesting "remote → local".
    let from = CGPoint(x: backRect.midX, y: backRect.maxY - h * 0.10)
    let to   = CGPoint(x: frontRect.minX + w * 0.18, y: frontRect.maxY - h * 0.18)
    ctx.saveGState()
    ctx.setStrokeColor(Palette.cream.cgColor)
    ctx.setLineWidth(size * 0.022)
    ctx.setLineCap(.round)
    ctx.beginPath()
    ctx.move(to: from)
    ctx.addLine(to: to)
    ctx.strokePath()
    // Arrowhead at `to`.
    let ang = atan2(to.y - from.y, to.x - from.x)
    let hs = size * 0.035
    let leftP  = CGPoint(x: to.x - cos(ang - .pi/6) * hs, y: to.y - sin(ang - .pi/6) * hs)
    let rightP = CGPoint(x: to.x - cos(ang + .pi/6) * hs, y: to.y - sin(ang + .pi/6) * hs)
    ctx.setFillColor(Palette.cream.cgColor)
    ctx.beginPath()
    ctx.move(to: to)
    ctx.addLine(to: leftP)
    ctx.addLine(to: rightP)
    ctx.closePath()
    ctx.fillPath()
    ctx.restoreGState()
}

func drawFolder(_ ctx: CGContext, rect: CGRect, body: NSColor, tab: NSColor) {
    let r = rect.height * 0.10
    // Tab on top.
    let tabRect = CGRect(x: rect.minX,
                         y: rect.maxY - rect.height * 0.18,
                         width: rect.width * 0.42,
                         height: rect.height * 0.16)
    ctx.saveGState()
    ctx.setFillColor(tab.cgColor)
    ctx.addPath(CGPath(roundedRect: tabRect, cornerWidth: r * 0.6, cornerHeight: r * 0.6, transform: nil))
    ctx.fillPath()

    // Folder body.
    let bodyRect = CGRect(x: rect.minX, y: rect.minY,
                          width: rect.width,
                          height: rect.height * 0.88)
    ctx.setFillColor(body.cgColor)
    ctx.addPath(CGPath(roundedRect: bodyRect, cornerWidth: r, cornerHeight: r, transform: nil))
    ctx.fillPath()
    ctx.restoreGState()
}

// MARK: - Variant 3b: folder-icon (workspace folder, Synology-style)
//
// Painted onto the workspace root directory via NSWorkspace.setIcon so Finder
// shows it in sidebar / column / list / icon views and the Get Info preview.
// Unlike the app-icon variants, this renders on a TRANSPARENT canvas: Finder
// expects a folder-shaped silhouette, not a squircle tile.
//
// Composition:
//   - A single macOS-blue folder shape filling most of the canvas.
//   - The twin-tiles glyph (two overlapping rounded squares, the Git-Same mark)
//     composited onto the front face of the folder, scaled to read at 32px.

func drawWorkspaceFolderIcon(_ ctx: CGContext, size: CGFloat) {
    // 1. Folder silhouette. Centered, with breathing room top/bottom so the
    //    tab doesn't kiss the canvas edge.
    let folderW = size * 0.86
    let folderH = size * 0.66
    let folderRect = CGRect(x: (size - folderW) / 2,
                            y: (size - folderH) / 2 - size * 0.02,
                            width: folderW, height: folderH)

    // Subtle drop shadow so the folder reads as an object on the desktop.
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -size * 0.012),
                  blur: size * 0.030,
                  color: NSColor.black.withAlphaComponent(0.30).cgColor)
    ctx.setFillColor(NSColor.black.withAlphaComponent(0.001).cgColor)
    ctx.fill(folderRect)
    ctx.restoreGState()

    // The tab + body, in macOS folder blue.
    let folderBlue = NSColor(srgbRed: 0x4A/255.0, green: 0x90/255.0, blue: 0xD9/255.0, alpha: 1)
    let folderBlueDark = folderBlue.blended(withFraction: 0.20, of: .black) ?? folderBlue
    drawFolder(ctx, rect: folderRect, body: folderBlue, tab: folderBlueDark)

    // 2. Twin-tiles glyph on the folder face. Smaller and centered horizontally,
    //    biased toward the bottom of the folder body so it reads as "on the
    //    front face" rather than crowding the tab.
    let bodyRect = CGRect(x: folderRect.minX, y: folderRect.minY,
                          width: folderRect.width,
                          height: folderRect.height * 0.88)
    let glyphScale: CGFloat = 0.62
    let glyphSide = bodyRect.height * glyphScale
    let glyphCX = bodyRect.midX
    let glyphCY = bodyRect.midY - bodyRect.height * 0.04
    let off = glyphSide * 0.15
    let tileSide = glyphSide * 0.78
    let r = tileSide * 0.26

    let backRect = CGRect(x: glyphCX - tileSide/2 - off,
                          y: glyphCY - tileSide/2 + off,
                          width: tileSide, height: tileSide)
    let frontRect = CGRect(x: glyphCX - tileSide/2 + off,
                           y: glyphCY - tileSide/2 - off,
                           width: tileSide, height: tileSide)

    // For folder-icon use we want the tiles to read clearly against the blue
    // folder body, so we use opaque cream + green/blue tints rather than the
    // translucent Liquid Glass treatment used by the app icon.
    drawSolidTile(ctx, rect: backRect, cornerRadius: r,
                  body: Palette.green, accent: Palette.green.blended(withFraction: 0.35, of: .black) ?? Palette.green,
                  excludeRect: frontRect, excludeCornerRadius: r)
    drawSolidTile(ctx, rect: frontRect, cornerRadius: r,
                  body: Palette.cream, accent: Palette.blue,
                  excludeRect: nil, excludeCornerRadius: 0)
}

/// Opaque variant of `drawGlassTile` used by the folder-icon variant. Reads
/// better against the saturated blue folder body than the translucent app-icon
/// glass.
func drawSolidTile(_ ctx: CGContext, rect: CGRect, cornerRadius r: CGFloat,
                   body: NSColor, accent: NSColor,
                   excludeRect: CGRect?, excludeCornerRadius: CGFloat) {
    let path = CGPath(roundedRect: rect, cornerWidth: r, cornerHeight: r, transform: nil)

    // Body fill.
    ctx.saveGState()
    ctx.addPath(path)
    ctx.clip()
    ctx.setFillColor(body.cgColor)
    ctx.fill(rect)

    // Repo rows, clipped away from the front tile when relevant.
    if let ex = excludeRect {
        ctx.saveGState()
        let outer = CGPath(rect: rect, transform: nil)
        let inner = CGPath(roundedRect: ex,
                           cornerWidth: excludeCornerRadius,
                           cornerHeight: excludeCornerRadius,
                           transform: nil)
        let combined = CGMutablePath()
        combined.addPath(outer)
        combined.addPath(inner)
        ctx.addPath(combined)
        ctx.clip(using: .evenOdd)
        drawRepoRows(ctx, in: rect, color: accent)
        ctx.restoreGState()
    } else {
        drawRepoRows(ctx, in: rect, color: accent)
    }
    ctx.restoreGState()

    // Hairline rim so adjacent tiles separate cleanly.
    ctx.saveGState()
    ctx.addPath(path)
    ctx.setStrokeColor(NSColor.white.withAlphaComponent(0.40).cgColor)
    ctx.setLineWidth(max(1, rect.width * 0.014))
    ctx.strokePath()
    ctx.restoreGState()
}

// MARK: - Variant 4: wordmark

func drawWordmark(_ ctx: CGContext, size: CGFloat) {
    drawGradientBackground(ctx, size: size)

    // Big monogram "gs" in cream, with an equals-style mirror cue between the letters.
    let fontSize = size * 0.58
    let font = NSFont.systemFont(ofSize: fontSize, weight: .black)
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: Palette.cream,
    ]
    let text = "gs"
    let bounds = textBounds(text, attrs: attrs)
    let pos = CGPoint(x: size/2 - bounds.width/2 - bounds.minX,
                      y: size/2 - bounds.height/2 - bounds.minY)
    drawAttributedText(ctx, text, attrs: attrs, at: pos)

    // Two short horizontal bars to the right, suggesting "=" (same).
    let barW = size * 0.16
    let barH = size * 0.045
    let barX = size * 0.78 - barW/2
    let barGap = size * 0.04
    ctx.saveGState()
    ctx.setFillColor(Palette.cream.cgColor)
    for y in [size/2 + barGap/2, size/2 - barGap/2 - barH] {
        let r = barH / 2
        ctx.addPath(CGPath(roundedRect: CGRect(x: barX, y: y, width: barW, height: barH),
                           cornerWidth: r, cornerHeight: r, transform: nil))
        ctx.fillPath()
    }
    ctx.restoreGState()
}

// MARK: - Variant 5: tui-banner

func drawTuiBanner(_ ctx: CGContext, size: CGFloat) {
    drawGradientBackground(ctx, size: size)

    // "g=s" rendered in a bold monospace, each glyph colored from the gradient stops.
    // Reads as a wink to the TUI ASCII banner.
    let fontSize = size * 0.46
    let font = NSFont.monospacedSystemFont(ofSize: fontSize, weight: .heavy)

    let glyphs: [(String, NSColor)] = [
        ("g", Palette.cream),
        ("=", Palette.cream),
        ("s", Palette.cream),
    ]

    // Measure full string to center.
    let fullAttrs: [NSAttributedString.Key: Any] = [
        .font: font, .foregroundColor: Palette.cream,
    ]
    let fullText = glyphs.map { $0.0 }.joined()
    let fullBounds = textBounds(fullText, attrs: fullAttrs)
    var x = size/2 - fullBounds.width/2 - fullBounds.minX
    let y = size/2 - fullBounds.height/2 - fullBounds.minY

    for (s, color) in glyphs {
        let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
        drawAttributedText(ctx, s, attrs: attrs, at: CGPoint(x: x, y: y))
        let b = textBounds(s, attrs: attrs)
        x += b.width
    }

    // Top and bottom hairline rules in cream, evoking the TUI banner's double-line frame.
    ctx.saveGState()
    ctx.setStrokeColor(Palette.cream.withAlphaComponent(0.85).cgColor)
    ctx.setLineWidth(size * 0.012)
    let inset = size * 0.16
    let topY = size * 0.78
    let botY = size * 0.22
    for ly in [topY, topY - size * 0.025, botY, botY + size * 0.025] {
        ctx.beginPath()
        ctx.move(to: CGPoint(x: inset, y: ly))
        ctx.addLine(to: CGPoint(x: size - inset, y: ly))
        ctx.strokePath()
    }
    ctx.restoreGState()
}

// MARK: - Dispatch

func renderVariant(_ name: String, size: Int) -> CGContext? {
    let s = CGFloat(size)
    let ctx = makeContext(size: size)

    // Default-clear background (transparent) so the squircle alpha shows.
    ctx.clear(CGRect(x: 0, y: 0, width: s, height: s))

    // Push an NSGraphicsContext so AppKit/CoreText drawing works.
    let nsCtx = NSGraphicsContext(cgContext: ctx, flipped: false)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = nsCtx
    defer { NSGraphicsContext.restoreGraphicsState() }

    switch name {
    case "twin":         drawTwin(ctx, size: s)
    case "sync":         drawSync(ctx, size: s)
    case "folder-pair":  drawFolderPair(ctx, size: s)
    case "folder-icon":  drawWorkspaceFolderIcon(ctx, size: s)
    case "wordmark":     drawWordmark(ctx, size: s)
    case "tui-banner":   drawTuiBanner(ctx, size: s)
    default:
        FileHandle.standardError.write(Data("unknown variant: \(name)\n".utf8))
        return nil
    }
    return ctx
}

// MARK: - Main

let args = parseArgs()
let variants = (args.variant == "all") ? allVariants : [args.variant]

for v in variants {
    guard let ctx = renderVariant(v, size: args.size) else { exit(2) }
    let path = "\(args.out)/\(v).png"
    do {
        try savePNG(ctx, to: path)
        print("wrote \(path)")
    } catch {
        FileHandle.standardError.write(Data("failed to write \(path): \(error)\n".utf8))
        exit(1)
    }
}
