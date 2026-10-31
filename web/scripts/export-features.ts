import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");

const sources = [
  ["batch", resolve(root, "docs/images/walkthrough/07-mixed-upload-progress.png")],
  ["annotate", resolve(root, "docs/images/walkthrough/image-editor.png")],
  ["compare", resolve(root, "docs/images/walkthrough/comparison-editor.png")],
] as const;
for (const [name, source] of sources) {
  for (const width of [620, 1240]) {
    const destination = resolve(root, `web/assets/images/feature-${name}-${width}.webp`);
    await new Bun.Image(Bun.file(source)).resize(width).webp({ lossless: true }).write(destination);
    console.log(`${name} (${width}px): ${Bun.file(destination).size} bytes`);
  }
}
