import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

test.skipIf(process.platform !== "darwin")(
  "native brand export produces opaque canvases and a readable multi-size favicon",
  async () => {
    const root = resolve(import.meta.dir, "..");
    const work = await mkdtemp(resolve(tmpdir(), "gatto-brand-test-"));
    try {
      const output = resolve(work, "public");
      await mkdir(output);
      const portrait = resolve(root, "web/public/favicon-32.png");
      for (const size of [16, 32, 64])
        await new Bun.Image(
          Bun.file(resolve(root, `web/public/favicon-${size === 16 ? 16 : 32}.png`)),
        )
          .resize(size, size, { filter: "nearest" })
          .png()
          .write(resolve(work, `favicon-${size}.png`));
      await new Bun.Image(Bun.file(portrait))
        .resize(160, 160, { filter: "nearest" })
        .png()
        .write(resolve(work, "apple-160.png"));
      await new Bun.Image(Bun.file(portrait))
        .resize(480, 480, { filter: "nearest" })
        .png()
        .write(resolve(work, "social-480.png"));
      const exported = Bun.spawnSync(
        ["swift", "packaging/export-brand-images.swift", work, output],
        { cwd: root },
      );
      expect(exported.exitCode, exported.stderr.toString()).toBe(0);
      for (const [name, width, height] of [
        ["apple-touch-icon.png", 180, 180],
        ["social-image-pixel.png", 1200, 630],
      ] as const)
        expect(await new Bun.Image(Bun.file(resolve(output, name))).metadata()).toEqual({
          width,
          height,
          format: "png",
        });

      // Read through Apple's decoder, independently checking the ICO's frames and
      // canvas alpha/background rather than relying on the writer's metadata.
      const inspected = Bun.spawnSync(["swift", "-", output], {
        stdin: new Blob([
          `
import Foundation
import CoreGraphics
import ImageIO
let directory = URL(fileURLWithPath: CommandLine.arguments[1])
let ico = CGImageSourceCreateWithURL(directory.appendingPathComponent("favicon.ico") as CFURL, nil)!
let sizes = (0..<CGImageSourceGetCount(ico)).map { CGImageSourceCreateImageAtIndex(ico, $0, nil)!.width }.sorted()
var corners: [[UInt8]] = []
for name in ["apple-touch-icon.png", "social-image-pixel.png"] {
    let source = CGImageSourceCreateWithURL(directory.appendingPathComponent(name) as CFURL, nil)!
    let image = CGImageSourceCreateImageAtIndex(source, 0, nil)!
    let context = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
        bytesPerRow: image.width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    corners.append(Array(UnsafeBufferPointer(start: context.data!.assumingMemoryBound(to: UInt8.self), count: 4)))
}
print(String(data: try JSONSerialization.data(withJSONObject: ["sizes": sizes, "corners": corners]), encoding: .utf8)!)
`,
        ]),
        stdout: "pipe",
        stderr: "pipe",
      });
      expect(inspected.exitCode, inspected.stderr.toString()).toBe(0);
      expect(JSON.parse(inspected.stdout.toString())).toEqual({
        sizes: [16, 32, 64],
        corners: [
          [247, 243, 233, 255],
          [247, 243, 233, 255],
        ],
      });
    } finally {
      await rm(work, { recursive: true, force: true });
    }
  },
);
