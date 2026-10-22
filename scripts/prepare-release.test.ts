import { expect, test } from "bun:test";
import { compareVersions, nextVersion, prepareChangelog, releaseNotes } from "./prepare-release";
import { bundleVersion, releaseOptions, selectReleaseVersion } from "./release-version";
import { mkdtemp, mkdir, chmod, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { releaseChecksums, verifyAsset } from "./verify-release";

const history =
  "# Changes\n\n## [Unreleased]\n\n### Fixed\n\n- Fix upload retry.\n\n## [0.14.0] - 2026-10-17\n\n### Added\n\n- Old feature.\n";
test("version choices stay stable and strictly advance the published version", () => {
  expect(nextVersion("0.14.0", "patch")).toBe("0.14.1");
  expect(nextVersion("0.14.0", "minor")).toBe("0.15.0");
  expect(nextVersion("0.14.0", "major")).toBe("1.0.0");
  for (const invalid of ["0.14.0", "0.13.0", "0.15.0-beta.0", "0.15.0-rc.1", "hello"])
    expect(() => nextVersion("0.14.0", invalid)).toThrow();
  expect(compareVersions("0.15.0", "0.14.9")).toBe(1);
});
test("beta versions advance numerically and promote to stable", () => {
  expect(bundleVersion("0.17.0-beta.1")).toBe("0.17.0b1");
  expect(bundleVersion("0.17.0")).toBe("0.17.0");
  expect(() => bundleVersion("0.17.0-beta.256")).toThrow();
  expect(nextVersion("0.16.3", "beta")).toBe("0.17.0-beta.1");
  expect(nextVersion("0.17.0-beta.9", "beta")).toBe("0.17.0-beta.10");
  expect(nextVersion("0.17.0-beta.10", "0.17.0")).toBe("0.17.0");
  expect(compareVersions("0.17.0-beta.10", "0.17.0-beta.9")).toBe(1);
  expect(compareVersions("0.17.0-beta.1", "0.16.3")).toBe(1);
  expect(compareVersions("0.17.0", "0.17.0-beta.10")).toBe(1);
  expect(
    prepareChangelog(
      history,
      "0.17.0-beta.1",
      "2026-10-22",
      releaseNotes(history, "0.17.0-beta.1"),
    ),
  ).toContain("## [0.17.0-beta.1]");
});

test("release options expose beta advancement and stable promotion with exact destinations", () => {
  const options = releaseOptions("0.17.0-beta.9", "0.16.3", "0.17.0-beta.9", false);
  expect(options.find((option) => option.key === "beta")?.version).toBe("0.17.0-beta.10");
  expect(options.find((option) => option.key === "stable")).toEqual({
    key: "stable",
    label: "Promote this Beta to Stable",
    version: "0.17.0",
  });
  expect(options.find((option) => option.key === "patch")?.version).toBe("0.17.1");
  expect(options.find((option) => option.key === "minor")?.version).toBe("0.18.0");
  expect(options.find((option) => option.key === "major")?.version).toBe("1.0.0");
  expect(nextVersion("0.17.0-beta.9", "stable")).toBe("0.17.0");
  expect(() => nextVersion("0.17.0", "stable")).toThrow();
  const limit = releaseOptions("0.17.0-beta.255", "0.16.3", "0.17.0-beta.255", false);
  expect(limit.some((option) => option.key === "beta")).toBe(false);
  expect(limit.find((option) => option.key === "stable")?.version).toBe("0.17.0");
  const stable = releaseOptions("0.17.0", "0.17.0", "0.17.0", false);
  expect(stable.find((option) => option.key === "beta")?.version).toBe("0.18.0-beta.1");
  expect(stable.some((option) => option.key === "stable")).toBe(false);
});

test("menu selections keep prepared versions and reject published or older targets", () => {
  const options = releaseOptions("0.17.0-beta.1", "0.16.3", "0.16.3", true);
  expect(options[0]?.key).toBe("prepared");
  const choose = (choice: string) =>
    selectReleaseVersion(options, choice, "0.17.0-beta.1", "0.16.3", "0.16.3");
  expect(choose("1")).toBe("0.17.0-beta.1");
  expect(choose("beta")).toBe("0.17.0-beta.2");
  expect(choose("promote")).toBe("0.17.0");
  expect(choose("stable")).toBe("0.17.0");
  expect(choose("0.18.0-beta.1")).toBe("0.18.0-beta.1");
  for (const invalid of ["0", "99", "hello", "0.16.3", "0.16.4"])
    expect(() => choose(invalid)).toThrow();
});

test("the interactive release script offers promotion, retries invalid choices and keeps its prepared default", async () => {
  const root = await mkdtemp(join(tmpdir(), "gatto-release-options-"));
  try {
    await mkdir(join(root, "scripts"));
    await mkdir(join(root, "packaging"));
    await mkdir(join(root, "tools"));
    for (const name of ["prepare-release.ts", "release-version.ts"])
      await Bun.write(
        join(root, "scripts", name),
        await Bun.file(new URL(`./${name}`, import.meta.url)).text(),
      );
    const gh = join(root, "tools/gh");
    await Bun.write(
      gh,
      `#!/bin/bash
if [[ "$1" == release ]]; then
  echo '{"tagName":"v0.16.3"}'
else
  echo '[[{"tag_name":"v0.17.0-beta.9","draft":false,"prerelease":true}]]'
fi
`,
    );
    await chmod(gh, 0o755);
    for (const { current, input, expected } of [
      { current: "0.17.0-beta.9", input: "invalid\npromote\n", expected: "0.17.0" },
      { current: "0.17.0-beta.9", input: "beta\n", expected: "0.17.0-beta.10" },
      { current: "0.18.0-beta.1", input: "\n", expected: "0.18.0-beta.1" },
    ]) {
      await Bun.write(
        join(root, "Cargo.toml"),
        `[package]\nname = "gatto"\nversion = "${current}"\n`,
      );
      await Bun.write(
        join(root, "packaging/Info.plist"),
        `<plist><dict><key>CFBundleVersion</key><string>${bundleVersion(current)}</string><key>CFBundleShortVersionString</key><string>${current}</string></dict></plist>`,
      );
      await Bun.write(join(root, "CHANGELOG.md"), history);
      const child = Bun.spawn(
        [process.execPath, "--no-env-file", join(root, "scripts/prepare-release.ts")],
        {
          cwd: root,
          env: { ...process.env, PATH: `${root}/tools:${process.env.PATH}` },
          stdin: new Blob([input]),
          stdout: "pipe",
          stderr: "pipe",
          timeout: 5000,
        },
      );
      const output = new Response(child.stdout).text();
      const errors = new Response(child.stderr).text();
      expect(await child.exited, (await output) + (await errors)).toBe(0);
      expect(await output).toContain("Promote this Beta to Stable");
      expect(await Bun.file(join(root, "Cargo.toml")).text()).toContain(`version = "${expected}"`);
      expect(await Bun.file(join(root, "packaging/Info.plist")).text()).toContain(
        `<string>${bundleVersion(expected)}</string>`,
      );
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
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
