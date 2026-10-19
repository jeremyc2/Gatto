// Refresh the in-repo tap from a published release; never guesses a checksum.
import { resolve } from "node:path";
import { parseArgs } from "node:util";

export function updateCask(source: string, version: string, checksums: string): string {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error("Expected a stable release version");
  const filename = `gatto-${version}-aarch64-apple-darwin.dmg`;
  const entries = checksums
    .trim()
    .split(/\r?\n/)
    .filter((line) => line.endsWith(`  ${filename}`));
  if (entries.length !== 1 || !/^[a-f0-9]{64}  /.test(entries[0]!)) {
    throw new Error(`Missing or ambiguous checksum for ${filename}`);
  }
  return source
    .replace(/^  version ".*"$/m, `  version "${version}"`)
    .replace(/^  sha256 ".*"$/m, `  sha256 "${entries[0]!.slice(0, 64)}"`)
    .replace(/\n  auto_updates true\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(
      /\n\n  depends_on /,
      `${/^[a-f0-9]{64}  appcast\.xml$/m.test(checksums) ? "\n\n  auto_updates true" : ""}\n\n  depends_on `,
    );
}

if (import.meta.main) {
  const root = resolve(import.meta.dir, "..");
  const { values } = parseArgs({
    options: { tag: { type: "string" }, check: { type: "boolean", default: false } },
  });
  if (values.tag && !/^v\d+\.\d+\.\d+$/.test(values.tag))
    throw new Error("Expected a stable vX.Y.Z tag");
  const result = Bun.spawnSync(
    [
      "gh",
      "release",
      "view",
      ...(values.tag ? [values.tag] : []),
      "--json",
      "tagName,isDraft,isPrerelease",
    ],
    { cwd: root },
  );
  if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  const release = JSON.parse(result.stdout.toString());
  if (release.isDraft || release.isPrerelease)
    throw new Error("The cask requires a published stable release");
  const tag = String(release.tagName);
  const sums = Bun.spawnSync(
    ["gh", "release", "download", tag, "--pattern", "SHA256SUMS", "--output", "-"],
    { cwd: root },
  );
  if (sums.exitCode !== 0) throw new Error(sums.stderr.toString());
  const cask = Bun.file(resolve(root, "Casks/gatto.rb"));
  const source = await cask.text();
  const updated = updateCask(source, tag.replace(/^v/, ""), sums.stdout.toString());
  if (values.check) {
    if (updated !== source) throw new Error(`Homebrew cask is stale for ${tag}`);
    console.log(`Homebrew cask matches ${tag}`);
  } else {
    await Bun.write(cask, updated);
    console.log(`Homebrew cask updated to ${tag}`);
  }
}
