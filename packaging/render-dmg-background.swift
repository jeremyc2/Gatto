#!/usr/bin/env swift
// Typeset the sole installation instruction over the original installer artwork.
// Usage: swift packaging/render-dmg-background.swift <output-directory> [--preview]
import AppKit

guard (2...3).contains(CommandLine.arguments.count),
      CommandLine.arguments.count == 2 || CommandLine.arguments[2] == "--preview" else {
    fatalError("Usage: render-dmg-background.swift <output-directory> [--preview]")
}
let preview = CommandLine.arguments.count == 3
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let assets = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
guard let artwork = NSImage(contentsOf: assets.appendingPathComponent("dmg-artwork.png")) else {
    fatalError("Missing dmg-artwork.png")
}
// Leave enough room for Finder's icon selection bounds so the window cannot
// pan past the artwork and expose a white strip on the right.
struct Layout: Decodable {
    let width: Int, height: Int, iconSize: Int
    let appX: Int, appY: Int, applicationsX: Int, applicationsY: Int
}
let layout = try JSONDecoder().decode(Layout.self,
    from: Data(contentsOf: assets.appendingPathComponent("dmg-layout.json")))
let width = layout.width, height = layout.height
// The preview uses the compiled app icon and macOS's actual shortcut icon.
// It renders the installer contents, without capturing a screen or a title bar.
let shortcut = output.appendingPathComponent("Applications")
if preview {
    try FileManager.default.createSymbolicLink(at: shortcut,
        withDestinationURL: URL(fileURLWithPath: "/Applications"))
}

func render(scale: Int, includeIcons: Bool) -> NSBitmapImageRep {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width * scale,
                                  pixelsHigh: height * scale, bitsPerSample: 8,
                                  samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                  colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    let context = NSGraphicsContext(bitmapImageRep: bitmap)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.cgContext.scaleBy(x: CGFloat(scale), y: CGFloat(scale))
    context.cgContext.translateBy(x: 0, y: CGFloat(height))
    context.cgContext.scaleBy(x: 1, y: -1)
    NSGraphicsContext.current = NSGraphicsContext(cgContext: context.cgContext, flipped: true)

    artwork.draw(in: NSRect(x: 0, y: 0, width: width, height: height), from: .zero,
                 operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)

    // Finder fixes picture-background labels to black. Fade the area beneath
    // the icons to black so their unselected names blend into the background.
    // Finder's real names remain accessible and appear when selected.
    let labelFade = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB),
                               colors: [NSColor.black.withAlphaComponent(0).cgColor,
                                        NSColor.black.cgColor] as CFArray, locations: [0, 1])!
    context.cgContext.drawLinearGradient(labelFade, start: CGPoint(x: 0, y: 350),
                                        end: CGPoint(x: 0, y: 380), options: [.drawsAfterEndLocation])
    let paragraph = NSMutableParagraphStyle()
    paragraph.alignment = .center
    ("Drag and Drop" as NSString).draw(
        in: NSRect(x: 0, y: 438, width: width, height: 42),
        withAttributes: [.font: NSFont.systemFont(ofSize: 28, weight: .semibold),
                         .foregroundColor: NSColor(srgbRed: 0.91, green: 0.95, blue: 1, alpha: 1),
                         .paragraphStyle: paragraph])

    if includeIcons {
        guard let appIcon = NSImage(contentsOf: assets.appendingPathComponent("AppIcon.icns")) else {
            fatalError("Missing compiled app icon")
        }
        for (icon, x, y) in [
            (appIcon, layout.appX, layout.appY),
            (NSWorkspace.shared.icon(forFile: shortcut.path), layout.applicationsX, layout.applicationsY)
        ] {
            icon.draw(in: NSRect(x: x - layout.iconSize / 2, y: y - layout.iconSize / 2,
                                width: layout.iconSize, height: layout.iconSize),
                      from: .zero, operation: .sourceOver, fraction: 1,
                      respectFlipped: true, hints: nil)
        }
    }
    NSGraphicsContext.restoreGraphicsState()
    // Apply logical size after rendering to avoid doubling the Retina transform.
    bitmap.size = NSSize(width: width, height: height)
    return bitmap
}

for scale in [1, 2] {
    let bitmap = render(scale: scale, includeIcons: false)
    try bitmap.representation(using: .png, properties: [:])!
        .write(to: output.appendingPathComponent("background\(scale == 1 ? "" : "@2x").png"))
    try bitmap.representation(using: .tiff, properties: [:])!
        .write(to: output.appendingPathComponent("background\(scale == 1 ? "" : "@2x").tiff"))
}
if preview {
    let bitmap = render(scale: 2, includeIcons: true)
    try bitmap.representation(using: .png, properties: [:])!
        .write(to: output.appendingPathComponent("installer@2x.png"))
    try FileManager.default.removeItem(at: shortcut)
}
