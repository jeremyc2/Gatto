import { $ } from "bun";
import { parseEnv } from "node:util";

const root = new URL("../../", import.meta.url);
const localEnv = Bun.file(new URL(".env.cloudflare", root));
const localValues = (await localEnv.exists()) ? parseEnv(await localEnv.text()) : {};
const isCloudflare = (name: string) => name.startsWith("CLOUDFLARE_") || name.startsWith("CF_");
const deployEnv = {
  ...Object.fromEntries(Object.entries(localValues).filter(([name]) => isCloudflare(name))),
  ...process.env,
};
const accountId = deployEnv.CLOUDFLARE_ACCOUNT_ID?.trim();
if (!accountId) {
  throw new Error(
    "Set CLOUDFLARE_ACCOUNT_ID in your environment or root .env.cloudflare before deploying.",
  );
}

const buildEnv = Object.fromEntries(
  Object.entries(process.env).filter(([name]) => !isCloudflare(name)),
);
const run = $.cwd(root.pathname).env(buildEnv);
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
await run.env(deployEnv)`cf deploy --prebuilt ${Bun.argv.slice(2)}`;
