import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
if (!Bun.which("cwebp")) throw new Error("Install the screenshot export tool: cwebp");

async function run(command: string[]) {
  const process = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
  if ((await process.exited) !== 0) throw new Error(`Screenshot export failed: ${command[0]}`);
}

const sources = [
  ["batch", resolve(root, "docs/images/walkthrough/07-mixed-upload-progress.png")],
  ["annotate", resolve(root, "docs/images/walkthrough/image-editor.png")],
  ["compare", resolve(root, "docs/images/walkthrough/comparison-editor.png")],
] as const;
for (const [name, source] of sources) {
  for (const width of [620, 1240]) {
    const destination = resolve(root, `web/assets/images/feature-${name}-${width}.webp`);
    await run([
      "cwebp",
      "-quiet",
      "-lossless",
      "-z",
      "9",
      "-resize",
      String(width),
      "0",
      source,
      "-o",
      destination,
    ]);
    console.log(`${name} (${width}px): ${Bun.file(destination).size} bytes`);
  }
}
