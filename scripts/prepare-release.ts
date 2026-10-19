// Interactive version/notes preparation used by release.sh. No publication here.
import { resolve } from "node:path";

const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
export function compareVersions(a: string, b: string): number {
  if (!versionPattern.test(a) || !versionPattern.test(b))
    throw new Error("Expected stable x.y.z versions");
  const left = a.split(".").map(Number);
  const right = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const difference = left[i]! - right[i]!;
    if (difference) return Math.sign(difference);
  }
  return 0;
}

export function nextVersion(previous: string, choice: string): string {
  compareVersions(previous, previous);
  const [major, minor, patch] = previous.split(".").map(Number);
  const result =
    choice === "major"
      ? `${major! + 1}.0.0`
      : choice === "minor"
        ? `${major}.${minor! + 1}.0`
        : choice === "patch"
          ? `${major}.${minor}.${patch! + 1}`
          : choice;
  if (compareVersions(result, previous) <= 0)
    throw new Error("Release version must be newer than the published release");
  return result;
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
  if (!versionPattern.test(version) || !/^\d{4}-\d{2}-\d{2}$/.test(date))
    throw new Error("Invalid release version/date");
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
    const locked = /\[\[package\]\]\nname = "gatto"\nversion = "([^"]+)"/.exec(
      await Bun.file(resolve(root, "Cargo.lock")).text(),
    )?.[1];
    const metadata = await plist.text();
    for (const key of ["CFBundleShortVersionString", "CFBundleVersion"]) {
      const version = new RegExp(`<key>${key}</key>\\s*<string>([^<]+)</string>`).exec(
        metadata,
      )?.[1];
      if (version !== current) throw new Error(`${key} does not match Cargo.toml`);
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
  const suggested = compareVersions(current, previous) > 0 ? current : "patch";
  const choice = prompt(
    `Latest published: v${previous}. Release version (patch=fixes, minor=features, major=breaking, or x.y.z)`,
    suggested,
  );
  if (choice === null) throw new Error("Release preparation cancelled");
  const version = nextVersion(previous, choice.trim());
  if (compareVersions(version, current) < 0)
    throw new Error("Cannot downgrade the version already prepared in Cargo.toml");
  const original = await changelog.text();
  let notes =
    compareVersions(current, previous) > 0
      ? releaseNotes(original, current)
      : (section(original, "Unreleased")?.body ?? "");
  if (!/^\s*- /m.test(notes)) {
    const summary = prompt("Release notes: describe the user-visible change in one sentence");
    if (!summary?.trim()) throw new Error("Release cancelled: release notes are required");
    const category = prompt(
      "Changelog category (Added/Changed/Deprecated/Removed/Fixed/Security)",
      choice === "minor" ? "Added" : "Fixed",
    );
    notes = `### ${category}\n\n- ${summary.trim()}`;
  }
  const date = new Intl.DateTimeFormat("en-CA", { timeZone: "America/Chicago" }).format(new Date());
  // Prepared notes can be moved to a newer version, but published history stays intact.
  let history = original;
  if (current !== version && compareVersions(current, previous) > 0) {
    const prepared = section(history, current);
    if (prepared) history = history.slice(0, prepared.start) + history.slice(prepared.end);
  }
  const updated = prepareChangelog(history, version, date, notes);
  const metadata = (await plist.text()).replace(
    /(<key>CFBundle(?:ShortVersionString|Version)<\/key>\s*<string>)[^<]+(<\/string>)/g,
    (_match: string, before: string, after: string) => before + version + after,
  );
  await Bun.write(manifest, manifestText.replace(/^version = "[^"]+"$/m, `version = "${version}"`));
  await Bun.write(plist, metadata);
  await Bun.write(changelog, updated);
  console.log(`Prepared v${version}:\n${notes}`);
}
