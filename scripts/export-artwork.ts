// Export approved masters without changing the editable artwork.
import { copyFile, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { parseEnv } from "node:util";

const root = resolve(import.meta.dir, "..");
const localEnv = Bun.file(resolve(root, ".env.aseprite"));
const aseprite =
  Bun.env.ASEPRITE_PATH ||
  ((await localEnv.exists()) ? parseEnv(await localEnv.text()).ASEPRITE_PATH : undefined);
if (!aseprite) throw new Error("Set ASEPRITE_PATH in .env.aseprite or your environment.");
for (const tool of [aseprite, "swift", "xcrun"])
  if (!Bun.which(tool)) throw new Error(`Install the artwork export tool: ${tool}`);

async function run(command: string[]) {
  const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (await child.exited) throw new Error(`Artwork export failed: ${command[0]}`);
}

const art = "docs/design/explorations/art";
const brand = `${art}/brand-cat`;
const work = await mkdtemp(resolve(tmpdir(), "gatto-artwork-"));
try {
  for (const [source, scale, destination] of [
    [`${brand}/brand-cat.aseprite`, 4, "docs/images/readme-cat.png"],
    [`${brand}/favicon-16.aseprite`, 1, "web/public/favicon-16.png"],
    [`${brand}/favicon-32.aseprite`, 1, "web/public/favicon-32.png"],
    [`${brand}/favicon-32.aseprite`, 3, "web/public/favicon-96.png"],
    [`${brand}/favicon-32.aseprite`, 2, `${work}/favicon-64.png`],
    [`${brand}/favicon-32.aseprite`, 5, `${work}/apple-160.png`],
    [`${brand}/menu-bar.aseprite`, 2, "packaging/menu-bar-icon.png"],
    [`${art}/app-cat/app-cat.aseprite`, 2, "packaging/app-cat-pixel.png"],
    [`${art}/portal-cat/portal-cat.aseprite`, 2, "web/assets/images/download-rocket-cat-pixel.png"],
    [`${art}/pixel-portal/portal.aseprite`, 2, "web/assets/images/download-portal-pixel.png"],
    [
      "docs/design/art/screenshot-cat/scout.aseprite",
      1,
      "docs/design/art/screenshot-cat/scout.png",
    ],
    [
      "docs/design/art/screenshot-cat/scout-black.aseprite",
      1,
      "docs/design/art/screenshot-cat/scout-black.png",
    ],
  ] as const) {
    await run([
      aseprite,
      "--batch",
      resolve(root, source),
      "--frame-range",
      "0,0",
      "--scale",
      String(scale),
      "--save-as",
      resolve(root, destination),
    ]);
  }
  await run([
    aseprite,
    "--batch",
    "--ignore-layer",
    "Warm paper",
    resolve(root, `${brand}/favicon-32.aseprite`),
    "--scale",
    "32",
    "--save-as",
    resolve(root, "packaging/AppIcon.icon/Assets/glyph.png"),
  ]);
  // Resize pixel art with nearest-neighbor sampling; Bun supplies the codecs.
  for (const size of [16, 32])
    await copyFile(resolve(root, `web/public/favicon-${size}.png`), `${work}/favicon-${size}.png`);
  await new Bun.Image(Bun.file(resolve(root, "web/public/favicon-96.png")))
    .resize(480, 480, { filter: "nearest" })
    .png()
    .write(`${work}/social-480.png`);
  await run(["swift", "packaging/export-brand-images.swift", work, resolve(root, "web/public")]);
  for (const name of ["apple-touch-icon.png", "social-image-pixel.png"]) {
    const path = resolve(root, "web/public", name);
    const bytes = await new Bun.Image(Bun.file(path)).png({ compressionLevel: 9 }).bytes();
    await Bun.write(path, bytes);
  }
  for (const name of ["download-rocket-cat-pixel", "download-portal-pixel"])
    await new Bun.Image(Bun.file(resolve(root, `web/assets/images/${name}.png`)))
      .webp({ lossless: true })
      .write(resolve(root, `web/assets/images/${name}.webp`));
  await new Bun.Image(Bun.file(resolve(root, "web/assets/images/portal-watercolor-paper.png")))
    .resize(1024, 1024)
    .webp({ quality: 90 })
    .write(resolve(root, "web/assets/images/portal-watercolor-paper.webp"));

  const compiled = resolve(work, "compiled");
  await mkdir(compiled);
  await run([
    "xcrun",
    "actool",
    "packaging/AppIcon.icon",
    "--compile",
    compiled,
    "--platform",
    "macosx",
    "--minimum-deployment-target",
    "13.0",
    "--app-icon",
    "AppIcon",
    "--output-partial-info-plist",
    resolve(work, "icon-info.plist"),
  ]);
  for (const file of ["AppIcon.icns", "Assets.car"])
    await copyFile(resolve(compiled, file), resolve(root, "packaging", file));
  const developer = Bun.spawnSync(["xcode-select", "-p"]);
  if (developer.exitCode) throw new Error("Select a full Xcode installation for Icon Composer.");
  const ictool = resolve(
    developer.stdout.toString().trim(),
    "../Applications/Icon Composer.app/Contents/Executables/ictool",
  );
  await run([
    ictool,
    "packaging/AppIcon.icon",
    "--export-image",
    "--output-file",
    "packaging/app-icon.png",
    "--platform",
    "macOS",
    "--rendition",
    "Default",
    "--width",
    "256",
    "--height",
    "256",
    "--scale",
    "1",
  ]);
} finally {
  await rm(work, { recursive: true, force: true });
}
