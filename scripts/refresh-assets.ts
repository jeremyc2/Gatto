// Shared preparation for app releases and website deployments. No publishing.
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
async function run(command: string[]) {
  const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (await child.exited) throw new Error(`Asset refresh failed: ${command.join(" ")}`);
}
const work = await mkdtemp(resolve(tmpdir(), "gatto-assets-"));
try {
  await run(["bun", "--no-env-file", "scripts/export-artwork.ts"]);
  await run(["bun", "--no-env-file", "export:story"]);
  // Icons and embedded art must exist before compiling the screenshot renderer.
  await run(["./scripts/update-walkthrough-screenshots.sh"]);
  await run(["swift", "packaging/render-dmg-background.swift", work, "--preview"]);
  await mkdir(resolve(root, "docs/images"), { recursive: true });
  await run([
    "cargo",
    "run",
    "--locked",
    "--quiet",
    "--features",
    "walkthrough-screenshots",
    "--bin",
    "walkthrough-screenshots",
    "--",
    "--frame-image",
    resolve(work, "installer@2x.png"),
    "docs/images/dmg-install.png",
  ]);
  await run(["bun", "--no-env-file", "export:features"]);
} finally {
  await rm(work, { recursive: true, force: true });
}
