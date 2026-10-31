import { expect, test } from "bun:test";
import { dirname, resolve } from "node:path";

test("Markdown embeds framed copies of every application screenshot", async () => {
  const root = resolve(import.meta.dir, "..");
  const files = Bun.spawnSync(
    ["git", "ls-files", "--cached", "--others", "--exclude-standard", "*.md"],
    { cwd: root },
  );
  expect(files.exitCode).toBe(0);
  let count = 0;
  for (const file of files.stdout.toString().trim().split("\n")) {
    const source = await Bun.file(resolve(root, file)).text();
    for (const match of source.matchAll(/!\[[^\]]*\]\(([^)]+)\)|<img\b[^>]*\bsrc="([^"]+)"/g)) {
      const path = match[1] ?? match[2]!;
      if (!path.includes("images/walkthrough/") && !path.endsWith("images/dmg-install.png"))
        continue;
      if (path.includes("images/walkthrough/"))
        expect(path, `${file}: use the framed screenshot`).toContain("/framed/");
      const image = new Bun.Image(Bun.file(resolve(root, dirname(file), path)));
      const metadata = await image.metadata();
      expect(metadata.format).toBe("png");
      const bytes = await Bun.file(resolve(root, dirname(file), path)).bytes();
      // The shared framer emits RGBA PNGs; IHDR byte 25 is the PNG color type.
      expect(bytes[25], `${file}: screenshot needs transparent rounded corners`).toBe(6);
      count++;
    }
  }
  expect(count).toBeGreaterThan(20);
});
