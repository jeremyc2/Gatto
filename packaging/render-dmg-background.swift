#!/usr/bin/env swift
// Typeset the sole installation instruction over the original installer artwork.
// Usage: swift packaging/render-dmg-background.swift <output-directory>
import AppKit

guard CommandLine.arguments.count == 2 else {
    fatalError("Usage: render-dmg-background.swift <output-directory>")
}
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let assets = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
guard let artwork = NSImage(contentsOf: assets.appendingPathComponent("dmg-artwork.png")) else {
    fatalError("Missing dmg-artwork.png")
}
let width = 800, height = 520

for scale in [1, 2] {
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

    // Finder fixes picture-background labels to black. The art fades to black
    // here, so native unselected names blend in. Plain light names below them
    // preserve contrast without glows, strokes, or panels. Finder's real names
    // remain accessible and appear normally on selection.
    let labelFade = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB),
                               colors: [NSColor.black.withAlphaComponent(0).cgColor,
                                        NSColor.black.cgColor] as CFArray, locations: [0, 1])!
    context.cgContext.drawLinearGradient(labelFade, start: CGPoint(x: 0, y: 350),
                                        end: CGPoint(x: 0, y: 380), options: [.drawsAfterEndLocation])
    let labelParagraph = NSMutableParagraphStyle()
    labelParagraph.alignment = .center
    for (label, x): (String, CGFloat) in [("Gatto", 110), ("Applications", 710)] {
        (label as NSString).draw(in: NSRect(x: x - 85, y: 413, width: 170, height: 30),
                                withAttributes: [.font: NSFont.systemFont(ofSize: 20),
                                                 .foregroundColor: NSColor(srgbRed: 0.91, green: 0.95,
                                                                           blue: 1, alpha: 1),
                                                 .paragraphStyle: labelParagraph])
    }

    let paragraph = NSMutableParagraphStyle()
    paragraph.alignment = .center
    ("Drag the app into the folder." as NSString).draw(
        in: NSRect(x: 0, y: 472, width: width, height: 32),
        withAttributes: [.font: NSFont.systemFont(ofSize: 22, weight: .medium),
                         .foregroundColor: NSColor(srgbRed: 0.91, green: 0.95, blue: 1, alpha: 1),
                         .paragraphStyle: paragraph])

    NSGraphicsContext.restoreGraphicsState()
    // Apply logical size after rendering to avoid doubling the Retina transform.
    bitmap.size = NSSize(width: width, height: height)
    try bitmap.representation(using: .png, properties: [:])!
        .write(to: output.appendingPathComponent("background\(scale == 1 ? "" : "@2x").png"))
    try bitmap.representation(using: .tiff, properties: [:])!
        .write(to: output.appendingPathComponent("background\(scale == 1 ? "" : "@2x").tiff"))
}
