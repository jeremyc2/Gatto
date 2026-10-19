// Check uploaded release assets against SHA256SUMS and, when present, local bytes.
import { mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { parseArgs } from "node:util";

export function releaseChecksums(source: string, version: string): Map<string, string> {
  const names = [`gatto-${version}-aarch64-apple-darwin.dmg`, "appcast.xml"];
  const entries = new Map<string, string>();
  for (const line of source.trim().split(/\r?\n/)) {
    const match = /^([a-f0-9]{64})  (.+)$/.exec(line);
    if (!match || !names.includes(match[2]!) || entries.has(match[2]!))
      throw new Error("Invalid or duplicate release checksum");
    entries.set(match[2]!, match[1]!);
  }
  if (entries.size !== names.length) throw new Error("Missing DMG or appcast checksum");
  return entries;
}

export function verifyAsset(
  name: string,
  bytes: ArrayBuffer,
  metadata: { size: number; digest?: string | null },
  expected?: string,
): string {
  const digest = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
  if (metadata.size !== bytes.byteLength) throw new Error(`Published asset size mismatch: ${name}`);
  if (metadata.digest && metadata.digest !== `sha256:${digest}`)
    throw new Error(`GitHub asset digest mismatch: ${name}`);
  if (expected && expected !== digest) throw new Error(`Published checksum mismatch: ${name}`);
  return digest;
}

if (import.meta.main) {
  const { values } = parseArgs({
    options: { tag: { type: "string" }, directory: { type: "string" } },
  });
  const tag = values.tag;
  if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error("Provide --tag vX.Y.Z");
  const directory = await mkdtemp(join(tmpdir(), "gatto-release-verify-"));
  const run = (args: string[]) => {
    const result = Bun.spawnSync(args);
    if (result.exitCode !== 0) throw new Error(result.stderr.toString());
    return result.stdout.toString();
  };
  try {
    const metadata = JSON.parse(
      run(["gh", "release", "view", tag, "--json", "assets,isPrerelease"]),
    );
    if (metadata.isPrerelease) throw new Error("Expected a stable release");
    run([
      "gh",
      "release",
      "download",
      tag,
      "--dir",
      directory,
      "--pattern",
      "*.dmg",
      "--pattern",
      "appcast.xml",
      "--pattern",
      "SHA256SUMS",
    ]);
    const checksums = releaseChecksums(
      await Bun.file(join(directory, "SHA256SUMS")).text(),
      tag.slice(1),
    );
    for (const name of [...checksums.keys(), "SHA256SUMS"]) {
      const file = Bun.file(join(directory, name));
      const bytes = await file.arrayBuffer();
      const assets = metadata.assets.filter((asset: { name: string }) => asset.name === name);
      if (assets.length !== 1) throw new Error(`Published asset name mismatch: ${name}`);
      const digest = verifyAsset(name, bytes, assets[0], checksums.get(name));
      if (values.directory) {
        const local = Bun.file(join(values.directory, name));
        const localDigest = new Bun.CryptoHasher("sha256")
          .update(await local.arrayBuffer())
          .digest("hex");
        if (localDigest !== digest)
          throw new Error(`Uploaded asset differs from the local build: ${name}`);
      }
    }
    console.log(`Verified ${tag}: uploaded DMG, signed-feed bytes and checksums match`);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
