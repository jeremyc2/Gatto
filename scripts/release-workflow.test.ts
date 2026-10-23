import { expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, chmod } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

// Execute the real shell workflow with isolated stand-ins for signing, GitHub,
// Git and build tools. Nothing can publish, sign with a real key, or push code.
async function workflow(mode: string, resume = false) {
  const root = await mkdtemp(join(tmpdir(), "gatto-release-workflow-"));
  const bin = join(root, "tools");
  const writeExecutable = async (name: string, source: string) => {
    await Bun.write(name, source);
    await chmod(name, 0o755);
  };
  try {
    for (const path of [bin, join(root, "scripts"), join(root, "packaging"), join(root, "Casks"), join(root, "docs/images/walkthrough")])
      await mkdir(path, { recursive: true });
    await Bun.write(
      join(root, "scripts/release.sh"),
      await Bun.file(new URL("./release.sh", import.meta.url)).text(),
    );
    await writeExecutable(
      join(root, "scripts/update-walkthrough-screenshots.sh"),
      await Bun.file(new URL("./update-walkthrough-screenshots.sh", import.meta.url)).text(),
    );
    await Bun.write(
      join(root, "Cargo.toml"),
      `[package]\nname = "gatto"\nversion = "${mode.startsWith("beta-") ? "0.17.0-beta.1" : "0.15.0"}"\n`,
    );
    await Bun.write(join(root, "packaging/Info.plist"), "test plist");
    await Bun.write(join(root, "packaging/AppIcon.icns"), "icon");
    await Bun.write(join(root, "packaging/Assets.car"), "assets");
    await mkdir(join(root, "packaging/Send to Gatto.workflow/Contents"), { recursive: true });
    await Bun.write(join(root, "packaging/Send to Gatto.workflow/Contents/Info.plist"), "quick action");
    await Bun.write(join(root, "README.md"), "old figures");
    await Bun.write(join(root, "Casks/gatto.rb"), "old cask");
    await Bun.write(join(root, "docs/images/walkthrough/image-editor.png"), "stale screenshot");
    const common = `
import { appendFileSync, mkdirSync, writeFileSync, cpSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
const root = process.env.GATTO_TEST_ROOT!;
const mode = process.env.GATTO_TEST_MODE!;
const args = process.argv.slice(2);
if (tool === 'bun' && args[0] === '--no-env-file') args.shift();
appendFileSync(join(root, 'calls'), JSON.stringify([tool, ...args]) + '\\n');
const dirty = join(root, 'dirty');
if ((tool === 'cargo' && ['check','clippy','test','build','run'].includes(args[0]!)) || (tool === 'bun' && ['install','audit:dependencies','typecheck:scripts','build:web','lint:web','typecheck:web','test:web','test'].includes(args[0]!))) {
 if (process.env.GH_TOKEN || process.env.CLOUDFLARE_API_TOKEN || process.env.APPLE_PASSWORD) appendFileSync(join(root,'build-exposures'), tool+' '+args.join(' ')+'\\n');
}
const draft = join(root, 'draft');
const sha = 'a'.repeat(40);
if (tool === 'git') {
 if (args[0] === 'branch') console.log('main');
 if (args[0] === 'rev-parse') {
  if (args.includes('refs/tags/v0.15.0')) process.exit(1);
  console.log(sha);
 }
 if (args[0] === 'diff' && args.includes('--cached') && args.includes('--quiet')) process.exit(existsSync(dirty) ? 1 : 0);
 if (args[0] === 'diff' && args.includes('--quiet') && mode === 'code-mismatch') process.exit(1);
 if (args[0] === 'push') {
  const countFile = join(root, 'push-count');
  const count = existsSync(countFile) ? Number(readFileSync(countFile,'utf8')) + 1 : 1;
  writeFileSync(countFile, String(count));
  if (mode === 'push-failure' && count === 2) process.exit(1);
 }
 if (args[0] === 'commit') writeFileSync(dirty, '');
 if (args[0] === 'commit') { const {unlinkSync} = await import('node:fs'); unlinkSync(dirty); }
} else if (tool === 'gh') {
 if (args[0] === 'release' && args[1] === 'view') {
  if (!args.includes('--json')) process.exit(1);
  if (args.includes('isDraft')) console.log(['draft-resume','beta-draft-resume'].includes(mode) ? 'true' : 'false');
  if (args.includes('targetCommitish')) console.log(sha);
 }
 if (args[1] === 'create') writeFileSync(draft, 'draft');
 if (args[1] === 'edit' && !existsSync(draft) && !['draft-resume','beta-draft-resume'].includes(mode)) throw new Error('Published without a verified draft');
} else if (tool === 'bun') {
 if (args[0] === 'scripts/prepare-release.ts' && args.includes('--print-notes')) console.log('### Added\\n\\n- Test release.');
 if (args[0] === 'scripts/verify-release.ts' && mode === 'verify-failure') process.exit(1);
 if (args[0] === 'typecheck:scripts' && mode === 'check-failure') process.exit(1);
 if (args[0] === 'audit:dependencies' && mode === 'audit-failure') process.exit(1);
 if (args[0] === 'scripts/update-homebrew-cask.ts' && mode === 'metadata-failure') process.exit(1);
 if (['scripts/update-readme-size.ts','scripts/update-homebrew-cask.ts'].includes(args[0]) && !args.includes('--check')) writeFileSync(dirty, 'metadata');
} else if (tool === 'cargo' && args[0] === 'run') {
 if (mode === 'screenshot-failure') process.exit(1);
 mkdirSync(args.at(-1)!, {recursive:true});
 writeFileSync(join(args.at(-1)!, 'image-editor.png'), 'fresh screenshot');
} else if (tool === 'cargo' && args[0] === 'build') {
 if (mode === 'build-failure') process.exit(1);
 mkdirSync(join(root,'target/aarch64-apple-darwin/release'),{recursive:true});
 writeFileSync(join(root,'target/aarch64-apple-darwin/release/gatto'),'binary');
} else if (tool === 'ditto') {
 if (args[0] === '-c') writeFileSync(args.at(-1)!, 'archive');
 else cpSync(args[0]!, args[1]!, {recursive:true});
}
`;
    for (const tool of [
      "git",
      "gh",
      "bun",
      "cargo",
      "rustup",
      "security",
      "codesign",
      "xcrun",
      "spctl",
      "ditto",
      "ruby",
      "plutil",
      "hdiutil",
      "diskutil",
    ]) {
      await writeExecutable(
        join(bin, tool),
        `#!${process.execPath}\nconst tool = ${JSON.stringify(tool)};\n${common}`,
      );
    }
    await writeExecutable(
      join(root, "scripts/prepare-updater.sh"),
      `#!/bin/bash
set -e
mkdir -p dist/sparkle/bin dist/sparkle/Sparkle.framework
printf bridge > dist/sparkle/libGattoUpdater.dylib
printf license > dist/sparkle/LICENSE
printf '#!/bin/bash\\nprintf test-public-key\\n' > dist/sparkle/bin/generate_keys
printf '#!/bin/bash\\nprintf feed > dist/release/appcast.xml\\n' > dist/sparkle/bin/generate_appcast
printf '#!/bin/bash\\nexit 0\\n' > dist/sparkle/bin/sign_update
chmod +x dist/sparkle/bin/*
`,
    );
    await writeExecutable(join(root, "scripts/test-updater.sh"), "#!/bin/bash\nexit 0\n");
    await writeExecutable(
      join(root, "scripts/publish-beta-feed.sh"),
      `#!/bin/bash
printf '["beta-feed","%s"]\\n' "$1" >> "$GATTO_TEST_ROOT/calls"
if [[ "$GATTO_TEST_MODE" == beta-feed-failure ]]; then exit 1; fi
`,
    );
    await writeExecutable(join(root, "scripts/test-platform.sh"), "#!/bin/bash\nexit 0\n");
    await writeExecutable(
      join(root, "scripts/build-dmg.sh"),
      '#!/bin/bash\nif [[ "$1" == --prepare ]]; then printf template > "$3"; else printf image > "$2"; fi\n',
    );
    await Bun.write(join(root, "dirty"), "code");
    if (resume) {
      await rm(join(root, "dirty"));
      await mkdir(join(root, "dist/release"), { recursive: true });
      for (const name of [
        `gatto-${mode.startsWith("beta-") ? "0.17.0-beta.1" : "0.15.0"}-aarch64-apple-darwin.dmg`,
        "appcast.xml",
        "SHA256SUMS",
      ])
        if (mode !== "published-no-artifacts")
          await Bun.write(join(root, "dist/release", name), "original artifact");
    }
    const child = Bun.spawn(
      ["/bin/bash", join(root, "scripts/release.sh"), ...(resume ? ["--resume"] : [])],
      {
        cwd: root,
        env: {
          ...process.env,
          GH_TOKEN: "gatto-fixture-not-a-real-token",
          CLOUDFLARE_API_TOKEN: "gatto-fixture-not-a-real-token",
          APPLE_PASSWORD: "gatto-fixture-not-a-real-password",
          PATH: `${bin}:${process.env.PATH}`,
          SIGN_IDENTITY: "Test Developer ID Application",
          GATTO_TEST_ROOT: root,
          GATTO_TEST_MODE: mode,
        },
        stdin: new Blob(["y\ny\n"]),
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const stdout = new Response(child.stdout).text();
    const stderr = new Response(child.stderr).text();
    const exit = await child.exited;
    const calls: string[][] = (await Bun.file(join(root, "calls")).text())
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const exposure = Bun.file(join(root, "build-exposures"));
    return {
      exit,
      calls,
      screenshot: await Bun.file(join(root, "docs/images/walkthrough/image-editor.png")).text(),
      output: (await stdout) + (await stderr),
      exposure: (await exposure.exists()) ? await exposure.text() : "",
      quickActionBundled: await Bun.file(join(root, "dist/release/Gatto.app/Contents/Resources/Send to Gatto.workflow/Contents/Info.plist")).exists(),
    };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

const find = (calls: string[][], ...prefix: string[]) =>
  calls.findIndex((call) => prefix.every((part, i) => call[i] === part));
test("release checks precede commits; uploads are verified as a draft before publication; metadata is verified and pushed", async () => {
  const result = await workflow("release");
  expect(result.exit, result.output).toBe(0);
  expect(result.exposure).toBe("");
  expect(result.quickActionBundled).toBe(true);
  const bundleQuickAction = find(result.calls, "ditto", "packaging/Send to Gatto.workflow");
  expect(bundleQuickAction).toBeGreaterThan(-1);
  expect(bundleQuickAction).toBeLessThan(find(result.calls, "codesign"));
  expect(find(result.calls, "bun", "typecheck:scripts")).toBeLessThan(
    find(result.calls, "git", "commit"),
  );
  const install = find(result.calls, "bun", "install");
  expect(result.calls[install]).toContain("--frozen-lockfile");
  expect(result.calls[install]).toContain("--ignore-scripts");
  expect(find(result.calls, "bun", "audit:dependencies")).toBeLessThan(
    find(result.calls, "git", "commit"),
  );
  expect(find(result.calls, "cargo", "build")).toBeLessThan(
    find(result.calls, "xcrun", "notarytool", "history"),
  );
  const create = find(result.calls, "gh", "release", "create");
  expect(result.calls[create]).toContain("--draft");
  expect(create).toBeLessThan(find(result.calls, "bun", "scripts/verify-release.ts"));
  expect(find(result.calls, "bun", "scripts/verify-release.ts")).toBeLessThan(
    find(result.calls, "gh", "release", "edit"),
  );
  const cask = result.calls.filter((call) => call[1] === "scripts/update-homebrew-cask.ts");
  expect(cask.length).toBe(2);
  expect(cask[0]).toContain("v0.15.0");
  expect(cask[1]).toContain("--check");
  expect(result.calls.filter((call) => call[0] === "git" && call[1] === "push").length).toBe(2);
});
test("failed checks prevent committing and publishing", async () => {
  const result = await workflow("check-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "git", "commit")).toBe(-1);
  expect(find(result.calls, "gh", "release", "create")).toBe(-1);
});
test("a failed dependency audit prevents committing, signing and publishing", async () => {
  const result = await workflow("audit-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "git", "commit")).toBe(-1);
  expect(find(result.calls, "codesign")).toBe(-1);
  expect(find(result.calls, "gh", "release", "create")).toBe(-1);
});
test("a failed final build never opens signing credentials or publishes", async () => {
  const result = await workflow("build-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "xcrun", "notarytool", "history")).toBe(-1);
  expect(find(result.calls, "codesign")).toBe(-1);
  expect(find(result.calls, "gh", "release", "create")).toBe(-1);
});
test("failed upload verification retains an unpublished draft", async () => {
  const result = await workflow("verify-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "gh", "release", "create")).toBeGreaterThan(-1);
  expect(find(result.calls, "gh", "release", "edit")).toBe(-1);
});
test("published release recovery only verifies and syncs metadata", async () => {
  const result = await workflow("published-resume", true);
  expect(result.exit, result.output).toBe(0);
  expect(find(result.calls, "cargo", "run")).toBe(-1);
  expect(result.screenshot).toBe("stale screenshot");
  expect(find(result.calls, "cargo", "build")).toBe(-1);
  expect(find(result.calls, "gh", "release", "create")).toBe(-1);
  expect(find(result.calls, "git", "add", "--", "README.md", "Casks/gatto.rb")).toBeGreaterThan(-1);
});
test("draft recovery uses its exact target commit and verifies reuploaded assets before publishing", async () => {
  const result = await workflow("draft-resume", true);
  expect(result.exit, result.output).toBe(0);
  expect(find(result.calls, "gh", "release", "upload")).toBeLessThan(
    find(result.calls, "bun", "scripts/verify-release.ts"),
  );
  expect(find(result.calls, "cargo", "build")).toBe(-1);
});
test("recovery refuses a checkout with code changes since the release", async () => {
  const result = await workflow("code-mismatch", true);
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "bun", "scripts/update-readme-size.ts")).toBe(-1);
});

test("published recovery can measure a downloaded DMG when local artifacts are absent", async () => {
  const result = await workflow("published-no-artifacts", true);
  expect(result.exit, result.output).toBe(0);
  const readme = result.calls.find((call) => call[1] === "scripts/update-readme-size.ts");
  expect(readme).not.toContain("--dmg");
});

test("metadata failures explain how to resume an already published release", async () => {
  const result = await workflow("metadata-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "gh", "release", "edit")).toBeGreaterThan(-1);
  expect(result.output).toContain("--resume");
});
test("a failed metadata push retains its local commit for recovery", async () => {
  const result = await workflow("push-failure");
  expect(result.exit).not.toBe(0);
  expect(result.calls.filter((call) => call[0] === "git" && call[1] === "commit").length).toBe(2);
  expect(result.output).toContain("metadata is committed locally");
  expect(result.output).toContain("--resume");
});

test("beta publication remains a prerelease and refreshes only the signed beta feed", async () => {
  const result = await workflow("beta-release");
  expect(result.exit, result.output).toBe(0);
  const create = result.calls[find(result.calls, "gh", "release", "create")]!;
  expect(create).toContain("v0.17.0-beta.1");
  expect(create).toContain("--prerelease");
  expect(create).toContain("--latest=false");
  const publish = result.calls[find(result.calls, "gh", "release", "edit")]!;
  expect(publish).toContain("--prerelease");
  expect(publish).toContain("--latest=false");
  expect(find(result.calls, "beta-feed", "v0.17.0-beta.1")).toBeGreaterThan(
    find(result.calls, "gh", "release", "edit"),
  );
  expect(find(result.calls, "bun", "scripts/update-homebrew-cask.ts")).toBe(-1);
  expect(find(result.calls, "bun", "scripts/update-readme-size.ts")).toBe(-1);
});
test("beta draft recovery preserves prerelease flags and the stable cask", async () => {
  const result = await workflow("beta-draft-resume", true);
  expect(result.exit, result.output).toBe(0);
  expect(result.calls[find(result.calls, "gh", "release", "edit")]).toContain("--prerelease");
  expect(result.calls[find(result.calls, "gh", "release", "edit")]).toContain("--latest=false");
  expect(find(result.calls, "bun", "scripts/update-homebrew-cask.ts")).toBe(-1);
});
test("feed publication failures retain a published release with a resume path", async () => {
  const result = await workflow("beta-feed-failure");
  expect(result.exit).not.toBe(0);
  expect(result.output).toContain("--resume");
  expect(find(result.calls, "gh", "release", "edit")).toBeGreaterThan(-1);
  expect(find(result.calls, "bun", "scripts/update-homebrew-cask.ts")).toBe(-1);
});

test("release regenerates walkthrough screenshots before committing", async () => {
  const result = await workflow("release");
  expect(result.exit, result.output).toBe(0);
  const capture = find(result.calls, "cargo", "run");
  expect(result.screenshot).toBe("fresh screenshot");
  expect(capture).toBeGreaterThan(-1);
  expect(result.calls[capture]).toContain("walkthrough-screenshots");
  expect(capture).toBeLessThan(find(result.calls, "git", "add", "--all"));
  expect(result.exposure).toBe("");
});
test("screenshot failure prevents committing, signing and publishing", async () => {
  const result = await workflow("screenshot-failure");
  expect(result.exit).not.toBe(0);
  expect(find(result.calls, "git", "commit")).toBe(-1);
  expect(find(result.calls, "codesign")).toBe(-1);
  expect(find(result.calls, "gh", "release", "create")).toBe(-1);
});
