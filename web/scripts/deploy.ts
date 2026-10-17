import { $ } from "bun";

const accountId = Bun.env.CLOUDFLARE_ACCOUNT_ID?.trim();
if (!accountId) {
  throw new Error("Set CLOUDFLARE_ACCOUNT_ID in your environment or root .env before deploying.");
}

const root = new URL("../../", import.meta.url);
const run = $.cwd(root.pathname);
const output = new URL(".cloudflare/output/v0/", root);

await run`bun run build:web`;
await run`rm -rf .cloudflare/output/v0`;
await run`mkdir -p .cloudflare/output/v0/workers/default`;
await run`cp -R web/dist/client .cloudflare/output/v0/workers/default/assets`;
await Bun.write(
  new URL("config.json", output),
  JSON.stringify({
    accountId,
    buildContext: { isPreview: false },
  }),
);
await Bun.write(
  new URL("workers/default/worker.config.json", output),
  JSON.stringify({
    name: "gatto",
    compatibilityDate: "2026-10-17",
    workersDev: true,
    previewUrls: false,
    assets: {},
  }),
);
await run`cf deploy --prebuilt ${Bun.argv.slice(2)}`;
