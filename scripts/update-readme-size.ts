#!/usr/bin/env bun
// Measure a published macOS release and refresh its README size paragraph.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  createReadStream,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const REPO = "jeremyc2/Gatto";
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const START = "<!-- release-size:start -->";
const END = "<!-- release-size:end -->";

type ReleaseAsset = {
  name: string;
  size: number;
  digest?: string | null;
};

type GitHubRelease = {
  tagName: string;
  url: string;
  isDraft: boolean;
  isPrerelease: boolean;
  assets: ReleaseAsset[];
};

function run(command: string, ...args: string[]): string {
  return execFileSync(command, args, { encoding: "utf8", stdio: "pipe" });
}

function isMounted(mount: string): boolean {
  try {
    return statSync(mount).dev !== statSync(dirname(mount)).dev;
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return false;
    throw error;
  }
}

function bundleBytes(directory: string): number {
  let bytes = 0;
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) bytes += bundleBytes(path);
    else if (entry.isFile()) bytes += statSync(path).size;
  }
  return bytes;
}

function measure(dmg: string, mount: string, tag: string): number {
  try {
    run(
      "diskutil",
      "image",
      "attach",
      "--readOnly",
      "--mountPoint",
      mount,
      "--mountOptions",
      "nobrowse",
      "--plist",
      dmg,
    );
    const app = join(mount, "Gatto.app");
    const version = run(
      "plutil",
      "-extract",
      "CFBundleShortVersionString",
      "raw",
      "-o",
      "-",
      join(app, "Contents/Info.plist"),
    ).trim();
    if (version !== tag.slice(1)) {
      throw new Error("bundle version does not match the release tag");
    }
    const archs = run("lipo", "-archs", join(app, "Contents/MacOS/gatto")).trim();
    if (archs !== "arm64") {
      throw new Error(`expected an Apple Silicon bundle, got ${archs}`);
    }
    return bundleBytes(app);
  } finally {
    // This mountpoint belongs to this invocation. Never eject other mounts.
    if (isMounted(mount)) run("diskutil", "eject", mount);
  }
}

async function main(): Promise<number> {
  const { values: args } = parseArgs({
    options: {
      tag: { type: "string" },
      dmg: { type: "string" },
      check: { type: "boolean", default: false },
      help: { type: "boolean", short: "h" },
    },
  });
  if (args.help) {
    console.log(`Measure a published macOS release and refresh its README size paragraph.

Usage: bun scripts/update-readme-size.ts [options]
  --tag <tag>    Release tag (default: latest published release)
  --dmg <path>   Reuse a DMG matching the published asset
  --check        Check figures without editing the README
  -h, --help     Show this help`);
    return 0;
  }
  if (process.platform !== "darwin") {
    throw new Error("measuring the app requires macOS");
  }

  const readme = join(ROOT, "README.md");
  const original = readFileSync(readme, "utf8");
  if (original.split(START).length !== 2 || original.split(END).length !== 2) {
    throw new Error("README must contain exactly one release-size marker pair");
  }
  const startIndex = original.indexOf(START) + START.length;
  const endIndex = original.indexOf(END);
  if (endIndex < startIndex) throw new Error("README release-size markers are out of order");

  const release: GitHubRelease = JSON.parse(
    run(
      "gh",
      "release",
      "view",
      ...(args.tag ? [args.tag] : []),
      "--repo",
      REPO,
      "--json",
      "tagName,assets,url,isDraft,isPrerelease",
    ),
  );
  if (release.isDraft || release.isPrerelease) {
    throw new Error("README figures require a published stable release");
  }
  const tag = release.tagName;
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) {
    throw new Error(`unsupported release tag: ${tag}`);
  }
  const name = `gatto-${tag.slice(1)}-aarch64-apple-darwin.dmg`;
  const assets = release.assets.filter((asset) => asset.name === name);
  if (assets.length !== 1) throw new Error(`expected one published asset named ${name}`);
  const asset = assets[0]!;

  const directory = mkdtempSync(join(tmpdir(), "gatto-release-size-"));
  const mount = join(directory, "mount");
  let downloadBytes: number;
  let installedBytes: number;
  try {
    run(
      "gh",
      "release",
      "download",
      tag,
      "--repo",
      REPO,
      "--dir",
      directory,
      "--pattern",
      "SHA256SUMS",
      ...(args.dmg ? [] : ["--pattern", name]),
    );
    const dmg = args.dmg
      ? resolve(args.dmg.startsWith("~/") ? join(homedir(), args.dmg.slice(2)) : args.dmg)
      : join(directory, name);
    const hashes = readFileSync(join(directory, "SHA256SUMS"), "utf8")
      .split(/\r?\n/)
      .flatMap((line) => {
        const match = line.match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
        return match && match[2] === name ? [match[1]!.toLowerCase()] : [];
      });
    if (hashes.length !== 1) {
      throw new Error(`SHA256SUMS must contain one valid checksum for ${name}`);
    }
    const hash = createHash("sha256");
    for await (const block of createReadStream(dmg)) hash.update(block);
    const digest = hash.digest("hex");
    if (digest !== hashes[0]) {
      throw new Error("DMG checksum does not match the published SHA256SUMS");
    }
    downloadBytes = statSync(dmg).size;
    if (downloadBytes !== asset.size) {
      throw new Error("DMG size does not match the published asset");
    }
    if (asset.digest && asset.digest !== `sha256:${digest}`) {
      throw new Error("DMG checksum does not match GitHub asset metadata");
    }
    mkdirSync(mount);
    installedBytes = measure(dmg, mount, tag);
  } finally {
    if (isMounted(mount)) {
      console.error(`Could not eject ${mount}; temporary files retained at ${directory}.`);
    } else {
      rmSync(directory, { recursive: true });
    }
  }

  const paragraph =
    `<a href="${release.url}">\n` +
    `  <img src="docs/images/download-macos.svg" alt="Download Gatto for macOS" width="380">\n` +
    `</a>\n\n` +
    `**Gatto ${tag}** — ` +
    `${(downloadBytes / 1_000_000).toFixed(1)} MB download · ` +
    `${(installedBytes / 1_000_000).toFixed(1)} MB installed app.`;
  const updated =
    original.slice(0, startIndex) + "\n" + paragraph + "\n" + original.slice(endIndex);
  console.log(
    `${tag}: ${downloadBytes.toLocaleString("en-US")} download bytes; ` +
      `${installedBytes.toLocaleString("en-US")} bundle bytes`,
  );
  if (args.check) {
    if (updated !== original) {
      console.error("README size figures are stale; run without --check.");
      return 1;
    }
    console.log("README size figures match the release.");
  } else if (updated !== original) {
    writeFileSync(readme, updated);
    console.log("Updated README size figures.");
  } else {
    console.log("README size figures already match the release.");
  }
  return 0;
}

try {
  process.exitCode = await main();
} catch (error) {
  console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}
