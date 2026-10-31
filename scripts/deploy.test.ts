import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

async function deploy(failRefresh: boolean) {
  const root = await mkdtemp(join(tmpdir(), "gatto-deploy-"));
  try {
    for (const path of ["web/scripts", "web/cloudflare", "tools"])
      await mkdir(join(root, path), { recursive: true });
    await Bun.write(
      join(root, "web/scripts/deploy.ts"),
      await Bun.file(new URL("../web/scripts/deploy.ts", import.meta.url)).text(),
    );
    await Bun.write(
      join(root, "web/cloudflare/worker.ts"),
      'export default { fetch() { return new Response("fixture"); } };',
    );
    for (const tool of ["bun", "cf"]) {
      const executable = join(root, "tools", tool);
      await Bun.write(
        executable,
        `#!${process.execPath}
import {appendFileSync, mkdirSync, writeFileSync, readFileSync} from 'node:fs';
const args = process.argv.slice(2);
appendFileSync('calls', JSON.stringify([${JSON.stringify(tool)}, ...args])+'\\n');
if (${JSON.stringify(tool)} === 'bun') {
  if (process.env.CLOUDFLARE_API_TOKEN) throw new Error('Build received deployment credentials');
  if (args.includes('scripts/refresh-assets.ts')) {
    if (${failRefresh}) process.exit(1);
    writeFileSync('refreshed', 'new screenshot');
  }
  if (args.includes('build:web')) {
    mkdirSync('web/dist/client', {recursive: true});
    writeFileSync('web/dist/client/screenshot', readFileSync('refreshed'));
  }
} else if (args.includes('--dry-run') && !process.env.CLOUDFLARE_API_TOKEN) {
  throw new Error('Deploy lost its credentials');
}
`,
      );
      await chmod(executable, 0o755);
    }
    const child = Bun.spawn([process.execPath, join(root, "web/scripts/deploy.ts"), "--dry-run"], {
      cwd: root,
      env: {
        ...Bun.env,
        PATH: `${join(root, "tools")}:${Bun.env.PATH}`,
        CLOUDFLARE_ACCOUNT_ID: "fixture",
        CLOUDFLARE_API_TOKEN: "not-a-real-token",
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [exit, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    const calls: string[][] = (await Bun.file(join(root, "calls")).text())
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    return {
      exit,
      output: stdout + stderr,
      calls,
      screenshot: await Bun.file(
        join(root, ".cloudflare/output/v0/workers/default/assets/screenshot"),
      )
        .text()
        .catch(() => ""),
    };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("deployment builds with fresh screenshots before invoking cf, forwarding dry-run", async () => {
  const result = await deploy(false);
  expect(result.exit, result.output).toBe(0);
  expect(result.calls).toEqual([
    ["bun", "--no-env-file", "scripts/refresh-assets.ts"],
    ["bun", "run", "build:web"],
    ["cf", "deploy", "--prebuilt", "--dry-run"],
  ]);
  expect(result.screenshot).toBe("new screenshot");
});

test("failed screenshot preparation stops deployment before building or contacting cf", async () => {
  const result = await deploy(true);
  expect(result.exit).not.toBe(0);
  expect(result.calls).toHaveLength(1);
  expect(result.screenshot).toBe("");
});
