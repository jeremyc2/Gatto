import { expect, test } from "bun:test";
import { chmod, mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

async function build(failure: string, args: string[] = []) {
  const root = await mkdtemp(join(tmpdir(), "gatto-local-build-"));
  try {
    for (const path of ["scripts", "tools", "packaging/licenses", "dist/local/Gatto.app"])
      await mkdir(join(root, path), { recursive: true });
    await Bun.write(
      join(root, "scripts/build-app.sh"),
      await Bun.file(new URL("./build-app.sh", import.meta.url)).text(),
    );
    for (const name of ["Info.plist", "AppIcon.icns", "Assets.car"])
      await Bun.write(join(root, "packaging", name), name);
    for (const name of ["Tonejs-LICENSE", "Visual-Effect-LICENSE"])
      await Bun.write(join(root, "packaging/licenses", name), name);
    await Bun.write(join(root, "dist/local/Gatto.app/previous"), "previous build");
    for (const tool of ["bun", "cargo", "codesign", "plutil", "git", "uname"]) {
      const path = join(root, "tools", tool);
      await Bun.write(
        path,
        `#!/bin/bash
set -eu
echo '${tool}' "$@" >> "$GATTO_BUILD_TEST_ROOT/calls"
[[ "$GATTO_BUILD_TEST_FAILURE" != '${tool}' ]] || exit 1
case '${tool}' in
  uname) if [[ "$1" == -m ]]; then echo arm64; else echo Darwin; fi ;;
  bun)
    mkdir -p .tools/video-encoders/arm64/{bin,notices}
    echo encoder > .tools/video-encoders/arm64/bin/ffmpeg
    echo license > .tools/video-encoders/arm64/notices/NOTICE
    ;;
  git) echo aaaaaaaaaaaa ;;
  cargo)
    profile=debug
    for arg in "$@"; do [[ "$arg" != --release ]] || profile=release; done
    mkdir -p "target/$profile"
    echo "$profile binary" > "target/$profile/Gatto"
    ;;
esac
`,
      );
      await chmod(path, 0o755);
    }
    const process = Bun.spawn(["/bin/bash", join(root, "scripts/build-app.sh"), ...args], {
      cwd: tmpdir(),
      env: {
        ...Bun.env,
        PATH: `${join(root, "tools")}:${Bun.env.PATH}`,
        GATTO_BUILD_TEST_ROOT: root,
        GATTO_BUILD_TEST_FAILURE: failure,
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [exit, stdout, stderr] = await Promise.all([
      process.exited,
      new Response(process.stdout).text(),
      new Response(process.stderr).text(),
    ]);
    return {
      exit,
      output: stdout + stderr,
      root,
      binary: await Bun.file(join(root, "dist/local/Gatto.app/Contents/MacOS/gatto"))
        .text()
        .catch(() => ""),
      notices: await Promise.all(
        ["Tonejs-LICENSE", "Visual-Effect-LICENSE"].map((name) =>
          Bun.file(join(root, "dist/local/Gatto.app/Contents/Resources", name))
            .text()
            .catch(() => ""),
        ),
      ),
      encoder: await Bun.file(join(root, "dist/local/Gatto.app/Contents/MacOS/gatto-video-ffmpeg"))
        .text()
        .catch(() => ""),
      probeExists: await Bun.file(
        join(root, "dist/local/Gatto.app/Contents/MacOS/gatto-video-ffprobe"),
      ).exists(),
      previous: await Bun.file(join(root, "dist/local/Gatto.app/previous")).exists(),
      calls: await Bun.file(join(root, "calls")).text(),
    };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("local build works from another directory and prints the fresh bundle path", async () => {
  const result = await build("");
  expect(result.exit, result.output).toBe(0);
  expect(result.binary).toBe("debug binary\n");
  expect(result.previous).toBe(false);
  expect(result.notices).toEqual(["Tonejs-LICENSE", "Visual-Effect-LICENSE"]);
  expect(result.encoder).toBe("encoder\n");
  expect(
    result.calls.indexOf("codesign --force --sign -", result.calls.indexOf("gatto-video-ffmpeg")),
  ).toBeGreaterThan(-1);
  expect(result.probeExists).toBe(false);
  expect(result.output).toContain(`Created ${result.root}/dist/local/Gatto.app`);
});

test("optimized local build uses the newly compiled release binary", async () => {
  const result = await build("", ["--release"]);
  expect(result.exit, result.output).toBe(0);
  expect(result.binary).toBe("release binary\n");
});

for (const failure of ["bun", "cargo", "codesign"]) {
  test(`failed ${failure} preserves the previous local app`, async () => {
    const result = await build(failure);
    expect(result.exit).not.toBe(0);
    expect(result.previous).toBe(true);
    expect(result.output).not.toContain("Created ");
    if (failure === "cargo") expect(result.calls).not.toContain("codesign");
  });
}
