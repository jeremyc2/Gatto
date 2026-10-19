import { expect, test } from "bun:test";
import { compareVersions, nextVersion, prepareChangelog, releaseNotes } from "./prepare-release";
import { releaseChecksums, verifyAsset } from "./verify-release";

const history =
  "# Changes\n\n## [Unreleased]\n\n### Fixed\n\n- Fix upload retry.\n\n## [0.14.0] - 2026-10-17\n\n### Added\n\n- Old feature.\n";
test("version choices stay stable and strictly advance the published version", () => {
  expect(nextVersion("0.14.0", "patch")).toBe("0.14.1");
  expect(nextVersion("0.14.0", "minor")).toBe("0.15.0");
  expect(nextVersion("0.14.0", "major")).toBe("1.0.0");
  for (const invalid of ["0.14.0", "0.13.0", "0.15.0-beta.1", "hello"])
    expect(() => nextVersion("0.14.0", invalid)).toThrow();
  expect(compareVersions("0.15.0", "0.14.9")).toBe(1);
});
test("release notes merge categories and preserve older release history", () => {
  const notes = releaseNotes(history, "0.15.0");
  expect(notes).not.toContain("Old feature");
  const prepared = prepareChangelog(history, "0.15.0", "2026-10-19", notes);
  expect(prepared).toContain("## [Unreleased]\n\n## [0.15.0] - 2026-10-19");
  expect(prepared.split("## [0.14.0]")[1]).toBe(history.split("## [0.14.0]")[1]);
  const merged = prepareChangelog(
    prepared,
    "0.15.0",
    "2026-10-19",
    notes + "\n\n### Added\n\n- Add updating.\n\n" + notes,
  );
  expect(merged.indexOf("### Added")).toBeLessThan(merged.indexOf("### Fixed"));
  expect(merged.match(/### Fixed/g)?.length).toBe(1);
  expect(() => prepareChangelog(history, "0.15.0", "2026-10-19", "")).toThrow();
});
test("publication verification requires unique checksums for both exact release assets", () => {
  const digest = "a".repeat(64);
  const sums = `${digest}  gatto-0.15.0-aarch64-apple-darwin.dmg\n${digest}  appcast.xml\n`;
  expect(releaseChecksums(sums, "0.15.0").size).toBe(2);
  for (const invalid of [
    "",
    sums + sums,
    sums.replace("0.15.0", "0.14.0"),
    sums.replace("appcast.xml", "other.xml"),
  ])
    expect(() => releaseChecksums(invalid, "0.15.0")).toThrow();
});

test("published bytes must match size, GitHub digest, and the checksum manifest", () => {
  const bytes = new TextEncoder().encode("abc").buffer;
  const digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
  expect(verifyAsset("update.dmg", bytes, { size: 3, digest: `sha256:${digest}` }, digest)).toBe(
    digest,
  );
  expect(() => verifyAsset("update.dmg", bytes, { size: 4 }, digest)).toThrow();
  expect(() =>
    verifyAsset("update.dmg", bytes, { size: 3, digest: "sha256:wrong" }, digest),
  ).toThrow();
  expect(() =>
    verifyAsset("update.dmg", new TextEncoder().encode("abd").buffer, { size: 3 }, digest),
  ).toThrow();
});
