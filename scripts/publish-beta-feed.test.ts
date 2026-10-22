import { expect, test } from "bun:test";
import { mkdtemp, mkdir, chmod, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

// Exercise the real publication/recovery shell with local GitHub/signing tools.
// Cryptographic merge behavior is covered separately by test-update-feed.sh.
async function publish(mode: "first" | "existing" | "network" | "invalid-signature") {
  const root = await mkdtemp(join(tmpdir(), "gatto-feed-workflow-"));
  try {
    for (const folder of ["scripts", "tools", "dist/sparkle/bin"])
      await mkdir(join(root, folder), { recursive: true });
    await Bun.write(
      join(root, "scripts/publish-beta-feed.sh"),
      await Bun.file(new URL("./publish-beta-feed.sh", import.meta.url)).text(),
    );
    const source = `
import {appendFileSync,copyFileSync,existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const root=process.env.GATTO_FEED_ROOT!;
const mode=process.env.GATTO_FEED_MODE!;
const args=process.argv.slice(2);
appendFileSync(join(root,'calls'),JSON.stringify([tool,...args])+'\\n');
const remote=join(root,'remote.xml');
if(tool==='gh') {
 if(args[0]==='release' && args[1]==='view') {
  if(args[2]==='beta') {
   if(mode==='network') { console.error('network failed'); process.exit(1); }
   if(!existsSync(remote)) { console.error('release not found'); process.exit(1); }
   console.log(JSON.stringify({isDraft:false,isPrerelease:true}));
  } else console.log('v0.16.3');
 }
 if(args[1]==='download') {
  const directory=args[args.indexOf('--dir')+1]!;
  mkdirSync(directory,{recursive:true});
  if(args[2]==='beta') copyFileSync(remote,join(directory,'appcast.xml'));
  else writeFileSync(join(directory,'appcast.xml'),args[2]!);
 }
 if(args[1]==='upload'||args[1]==='create') {
  if(args[1]==='create' && (!args.includes('--prerelease')||!args.includes('--latest=false'))) throw new Error('unsafe feed publication');
  copyFileSync(args[3]!,remote);
 }
} else if(tool==='sign_update') {
 if(mode==='invalid-signature' && args.includes('--verify')) process.exit(1);
} else if(tool==='xcrun') {
 if(args[0]!=='swift') throw new Error('unexpected executable');
 writeFileSync(args[2]!, 'combined-signed-feed-fixture');
} else if(tool==='git') console.log('a'.repeat(40));
`;
    for (const tool of ["gh", "xcrun", "git", "sign_update"]) {
      const file = join(
        root,
        tool === "sign_update" ? "dist/sparkle/bin/sign_update" : `tools/${tool}`,
      );
      await Bun.write(
        file,
        `#!${process.execPath}\nconst tool=${JSON.stringify(tool)};\n${source}`,
      );
      await chmod(file, 0o755);
    }
    if (mode === "existing") await Bun.write(join(root, "remote.xml"), "previous-signed-feed");
    const child = Bun.spawn(
      ["/bin/bash", join(root, "scripts/publish-beta-feed.sh"), "v0.17.0-beta.1"],
      {
        cwd: root,
        env: {
          ...process.env,
          PATH: `${root}/tools:${process.env.PATH}`,
          GATTO_FEED_ROOT: root,
          GATTO_FEED_MODE: mode,
        },
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const output = new Response(child.stdout).text();
    const errors = new Response(child.stderr).text();
    const status = await child.exited;
    const calls: string[][] = (await Bun.file(join(root, "calls")).text())
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    return { status, calls, output: (await output) + (await errors) };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("first and repeated beta feed publication verifies before and after uploading", async () => {
  for (const mode of ["first", "existing"] as const) {
    const result = await publish(mode);
    expect(result.status, result.output).toBe(0);
    const uploads = result.calls.filter(
      (call) => call[0] === "gh" && ["upload", "create"].includes(call[2]!),
    );
    expect(uploads.length).toBe(1);
    expect(uploads[0]?.[2]).toBe(mode === "first" ? "create" : "upload");
    const verifies = result.calls.filter(
      (call) => call[0] === "sign_update" && call.includes("--verify"),
    );
    expect(verifies.length).toBe(mode === "first" ? 4 : 5);
    expect(result.calls.at(-1)?.[0]).toBe("sign_update");
  }
});
test("missing feed is distinguished from network and signature failures", async () => {
  for (const mode of ["network", "invalid-signature"] as const) {
    const result = await publish(mode);
    expect(result.status).not.toBe(0);
    expect(
      result.calls.some((call) => call[0] === "gh" && ["upload", "create"].includes(call[2]!)),
    ).toBe(false);
  }
});
