// Interactive version/notes preparation used by release.sh. No publication here.
import { resolve } from "node:path";

import {
  compareVersions,
  releaseChannel,
  bundleVersion,
  releaseOptions,
  selectReleaseVersion,
} from "./release-version";
export { compareVersions, nextVersion } from "./release-version";

export function prepareLockfile(source: string, version: string): string {
  bundleVersion(version);
  const packageVersion = /(^\[\[package\]\]\r?\nname = "gatto"\r?\nversion = ")[^"]+("\r?$)/m;
  if (!packageVersion.test(source)) throw new Error("Missing Gatto package in Cargo.lock");
  // Change only the local package version; preserve every dependency pin and checksum.
  return source.replace(
    packageVersion,
    (_match, before: string, after: string) => before + version + after,
  );
}

function section(source: string, version: string) {
  const headings = [...source.matchAll(/^## \[([^\]]+)\](?: - \d{4}-\d{2}-\d{2})?[ \t]*$/gm)];
  const index = headings.findIndex((match) => match[1] === version);
  if (index === -1) return undefined;
  const heading = headings[index]!;
  const start = heading.index!;
  const bodyStart = start + heading[0].length;
  const end = headings[index + 1]?.index ?? source.length;
  return { start, end, body: source.slice(bodyStart, end).trim() };
}

export function releaseNotes(source: string, version: string): string {
  return [section(source, version)?.body, section(source, "Unreleased")?.body]
    .filter(Boolean)
    .join("\n\n");
}

export function prepareChangelog(
  source: string,
  version: string,
  date: string,
  notes: string,
): string {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) throw new Error("Invalid release version/date");
  bundleVersion(version);
  const categories = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"];
  const groups = new Map<string, string[]>();
  let category: string | undefined;
  for (const line of notes.split("\n")) {
    const heading = /^### (.+)$/.exec(line);
    if (heading) {
      category = heading[1]!;
      if (!categories.includes(category))
        throw new Error(`Unknown changelog category: ${category}`);
      if (!groups.has(category)) groups.set(category, []);
    } else if (category) {
      groups.get(category)!.push(line);
    } else if (line.trim()) throw new Error("Release notes must use Keep a Changelog categories");
  }
  const body = categories
    .flatMap((name) => {
      const text = groups.get(name)?.join("\n").trim();
      return text && /^- /m.test(text) ? [`### ${name}\n\n${text}`] : [];
    })
    .join("\n\n");
  if (!body) throw new Error("Release notes must contain at least one user-visible change");
  const unreleased = section(source, "Unreleased");
  if (!unreleased) throw new Error("Missing Unreleased section");
  let result = source;
  const existing = section(source, version);
  if (existing) result = result.slice(0, existing.start) + result.slice(existing.end);
  const pending = section(result, "Unreleased")!;
  result =
    result.slice(0, pending.start) +
    `## [Unreleased]\n\n## [${version}] - ${date}\n\n${body}\n\n` +
    result.slice(pending.end);
  return result;
}

if (import.meta.main) {
  const root = resolve(import.meta.dir, "..");
  const manifest = Bun.file(resolve(root, "Cargo.toml"));
  const lockfile = Bun.file(resolve(root, "Cargo.lock"));
  const plist = Bun.file(resolve(root, "packaging/Info.plist"));
  const changelog = Bun.file(resolve(root, "CHANGELOG.md"));
  const manifestText = await manifest.text();
  const current = /^version = "([^"]+)"$/m.exec(manifestText)?.[1];
  if (!current) throw new Error("Missing Cargo package version");
  if (process.argv.includes("--print-notes")) {
    const notes = section(await changelog.text(), current)?.body;
    if (!notes) throw new Error(`Missing release notes for ${current}`);
    console.log(notes);
    process.exit(0);
  }
  if (process.argv.includes("--check")) {
    releaseChannel(current);
    const locked = /\[\[package\]\]\nname = "gatto"\nversion = "([^"]+)"/.exec(
      await Bun.file(resolve(root, "Cargo.lock")).text(),
    )?.[1];
    const metadata = await plist.text();
    for (const key of ["CFBundleShortVersionString", "CFBundleVersion"]) {
      const version = new RegExp(`<key>${key}</key>\\s*<string>([^<]+)</string>`).exec(
        metadata,
      )?.[1];
      if (version !== (key === "CFBundleVersion" ? bundleVersion(current) : current))
        throw new Error(`${key} does not match Cargo.toml`);
    }
    if (locked !== current) throw new Error("Cargo.lock does not match Cargo.toml");
    const notes = section(await changelog.text(), current)?.body;
    if (!notes || !/^- /m.test(notes)) throw new Error(`Missing release notes for ${current}`);
    console.log(`Validated version, lockfile, bundle and changelog for v${current}`);
    process.exit(0);
  }
  const published = Bun.spawnSync(["gh", "release", "view", "--json", "tagName"], { cwd: root });
  if (published.exitCode !== 0)
    throw new Error("Cannot establish the last published release; check GitHub access");
  const previous = String(JSON.parse(published.stdout.toString()).tagName).replace(/^v/, "");
  // Include prereleases when preparing the next beta; GitHub's latest endpoint
  // intentionally remains the stable baseline.
  if (releaseChannel(previous) !== "stable")
    throw new Error("GitHub's latest release must remain Stable");
  const listed = Bun.spawnSync(
    ["gh", "api", "--paginate", "--slurp", "repos/jeremyc2/Gatto/releases?per_page=100"],
    { cwd: root },
  );
  if (listed.exitCode !== 0) throw new Error("Cannot establish published beta versions");
  const pages: { tag_name: string; draft: boolean; prerelease: boolean }[][] = JSON.parse(
    listed.stdout.toString(),
  );
  const releases = pages.flat().map((release) => ({
    tagName: release.tag_name,
    isDraft: release.draft,
    isPrerelease: release.prerelease,
  }));
  let latest = previous;
  for (const release of releases) {
    const candidate = release.tagName.replace(/^v/, "");
    if (release.isDraft || !/^\d+\.\d+\.\d+(?:-beta\.[1-9]\d*)?$/.test(candidate)) continue;
    if ((releaseChannel(candidate) === "beta") !== release.isPrerelease)
      throw new Error(`Release channel disagrees with ${release.tagName}`);
    if (compareVersions(candidate, latest) > 0) latest = candidate;
  }
  const isPrepared =
    current !== previous &&
    !releases.some((release) => !release.isDraft && release.tagName === `v${current}`);
  const options = releaseOptions(current, previous, latest, isPrepared);
  const suggested =
    options.find((option) => option.key === "prepared") ??
    options.find(
      (option) => option.key === (releaseChannel(current) === "beta" ? "beta" : "patch"),
    ) ??
    options[0]!;
  console.log(
    `\nCurrent: v${current}\nLatest Stable: v${previous}\nLatest published overall: v${latest}\n\nChoose a release:`,
  );
  for (const [index, option] of options.entries())
    console.log(
      `  ${index + 1}. ${option.label} → ${option.version} (${releaseChannel(option.version) === "beta" ? "Beta" : "Stable"}) [${option.key}]`,
    );
  console.log(`  ${options.length + 1}. Custom version… [custom]\n`);
  let version: string;
  let selectedChoice: string;
  for (;;) {
    const choice = prompt("Enter a number, option name, or explicit version", suggested.key);
    if (choice === null) throw new Error("Release preparation cancelled");
    selectedChoice = choice.trim().toLowerCase();
    let input = choice;
    if (selectedChoice === "custom" || selectedChoice === String(options.length + 1)) {
      const custom = prompt("Custom version (x.y.z for Stable, x.y.z-beta.N for Beta)");
      if (custom === null) throw new Error("Release preparation cancelled");
      input = custom;
    }
    try {
      version = selectReleaseVersion(options, input, current, previous, latest);
      selectedChoice = options.find((option) => option.version === version)?.key ?? selectedChoice;
      break;
    } catch (error) {
      console.error(error instanceof Error ? error.message : "Invalid release choice");
    }
  }
  console.log(
    `Selected ${releaseChannel(version) === "beta" ? "Beta prerelease" : "Stable release"}: v${version}`,
  );
  const original = await changelog.text();
  let notes = isPrepared
    ? releaseNotes(original, current)
    : (section(original, "Unreleased")?.body ?? "");
  if (!/^\s*- /m.test(notes)) {
    const summary = prompt("Release notes: describe the user-visible change in one sentence");
    if (!summary?.trim()) throw new Error("Release cancelled: release notes are required");
    const category = prompt(
      "Changelog category (Added/Changed/Deprecated/Removed/Fixed/Security)",
      selectedChoice === "minor" ? "Added" : "Fixed",
    );
    notes = `### ${category}\n\n- ${summary.trim()}`;
  }
  const date = new Intl.DateTimeFormat("en-CA", { timeZone: "America/Chicago" }).format(new Date());
  // Prepared notes can be moved to a newer version, but published history stays intact.
  let history = original;
  if (current !== version && isPrepared) {
    const prepared = section(history, current);
    if (prepared) history = history.slice(0, prepared.start) + history.slice(prepared.end);
  }
  const updated = prepareChangelog(history, version, date, notes);
  const metadata = (await plist.text()).replace(
    /(<key>CFBundle(?:ShortVersionString|Version)<\/key>\s*<string>)[^<]+(<\/string>)/g,
    (_match: string, before: string, after: string) =>
      before +
      (before.includes("<key>CFBundleVersion</key>") ? bundleVersion(version) : version) +
      after,
  );
  const locked = prepareLockfile(await lockfile.text(), version);
  await Bun.write(manifest, manifestText.replace(/^version = "[^"]+"$/m, `version = "${version}"`));
  await Bun.write(lockfile, locked);
  await Bun.write(plist, metadata);
  await Bun.write(changelog, updated);
  console.log(`Prepared v${version}:\n${notes}`);
}
