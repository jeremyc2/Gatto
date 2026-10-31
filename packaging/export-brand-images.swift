// Native image container/canvas operations not exposed by Bun.Image.
// Usage: swift packaging/export-brand-images.swift <temporary-inputs> <public-directory>
import Foundation
import CoreGraphics
import ImageIO

guard CommandLine.arguments.count == 3 else {
    fatalError("Usage: export-brand-images.swift <temporary-inputs> <public-directory>")
}
let inputs = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let output = URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)

func image(_ url: URL) throws -> CGImage {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(domain: "BrandImages", code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Could not decode \(url.path)"])
    }
    return image
}

func write(_ images: [CGImage], type: String, to url: URL) throws {
    guard let destination = CGImageDestinationCreateWithURL(url as CFURL, type as CFString, images.count, nil) else {
        throw NSError(domain: "BrandImages", code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Cannot encode \(type)"])
    }
    for image in images { CGImageDestinationAddImage(destination, image, nil) }
    guard CGImageDestinationFinalize(destination) else {
        throw NSError(domain: "BrandImages", code: 3,
            userInfo: [NSLocalizedDescriptionKey: "Could not write \(url.path)"])
    }
}

func centered(_ source: CGImage, width: Int, height: Int) -> CGImage {
    // Opaque sRGB warm paper, matching the approved website/brand background.
    let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
        bytesPerRow: width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    context.setFillColor(red: 247.0 / 255, green: 243.0 / 255, blue: 233.0 / 255, alpha: 1)
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    context.interpolationQuality = .none
    context.draw(source, in: CGRect(x: (width - source.width) / 2,
        y: (height - source.height) / 2, width: source.width, height: source.height))
    return context.makeImage()!
}

let icons = try [16, 32, 64].map { try image(inputs.appendingPathComponent("favicon-\($0).png")) }
try write(icons, type: "com.microsoft.ico", to: output.appendingPathComponent("favicon.ico"))
try write([centered(image(inputs.appendingPathComponent("apple-160.png")), width: 180, height: 180)],
    type: "public.png", to: output.appendingPathComponent("apple-touch-icon.png"))
try write([centered(image(inputs.appendingPathComponent("social-480.png")), width: 1200, height: 630)],
    type: "public.png", to: output.appendingPathComponent("social-image-pixel.png"))
