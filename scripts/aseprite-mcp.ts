import { existsSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { parseEnv } from "node:util";
import { checkAseprite } from "./aseprite-mcp-check";

// Review upstream guidance and rerun the smoke check when changing this revision.
const revision = "a89b205bed31930ee141edf319e78c71ef16bba6";
const upstream = "https://github.com/diivi/aseprite-mcp.git";
const root = resolve(import.meta.dir, "..");
const checkout = resolve(root, ".tools/aseprite-mcp");
const mode = process.argv[2] ?? "--stdio";
const environment = Object.fromEntries(
  Object.entries(process.env).filter(
    ([name]) => !name.startsWith("CLOUDFLARE_") && !name.startsWith("CF_"),
  ),
);

function capture(command: string[]): string {
  const result = Bun.spawnSync(command, { cwd: root, env: environment });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString().trim());
  return result.stdout.toString().trim();
}

async function run(command: string[], env = environment): Promise<void> {
  const child = Bun.spawn(command, {
    cwd: root,
    env,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  const exitCode = await child.exited;
  if (exitCode !== 0) throw new Error(`${command[0]} exited with status ${exitCode}`);
}

async function main(): Promise<void> {
  if (process.argv.length > 3 || !["--setup", "--stdio", "--check"].includes(mode)) {
    throw new Error("Usage: bun --no-env-file scripts/aseprite-mcp.ts [--setup|--stdio|--check]");
  }
  const uv = Bun.which("uv") ?? Bun.which("uv", { PATH: `${process.env.HOME}/.local/bin` });
  if (!uv) throw new Error("Install uv: https://docs.astral.sh/uv/getting-started/installation/");
  const localEnv = Bun.file(resolve(root, ".env.aseprite"));
  const localAsepritePath = (await localEnv.exists())
    ? parseEnv(await localEnv.text()).ASEPRITE_PATH
    : undefined;
  const aseprite = process.env.ASEPRITE_PATH || localAsepritePath;
  if (!aseprite) {
    throw new Error(
      "Set ASEPRITE_PATH in .env.aseprite or your environment to the Aseprite executable.",
    );
  }
  if (!existsSync(aseprite))
    throw new Error("ASEPRITE_PATH does not point to an existing Aseprite executable.");
  const env = { ...environment, ASEPRITE_PATH: aseprite };

  if (mode === "--setup") {
    capture([aseprite, "--version"]);
    if (!existsSync(checkout)) {
      mkdirSync(resolve(root, ".tools"), { recursive: true });
      await run(["git", "clone", "--filter=blob:none", "--no-checkout", upstream, checkout]);
      await run(["git", "-C", checkout, "checkout", "--detach", revision]);
    }
    if (capture(["git", "-C", checkout, "status", "--porcelain"])) {
      throw new Error(
        `Keep local edits in ${checkout} before updating; setup requires a clean checkout.`,
      );
    }
    if (capture(["git", "-C", checkout, "rev-parse", "HEAD"]) !== revision) {
      await run(["git", "-C", checkout, "fetch", "origin", revision]);
      await run(["git", "-C", checkout, "checkout", "--detach", revision]);
    }
    await run([uv, "--directory", checkout, "sync", "--locked", "--no-dev"], env);
    console.log(
      "Aseprite MCP installed. Run bun aseprite:check to verify the connection and exports.",
    );
    return;
  }

  if (!existsSync(resolve(checkout, ".venv/bin/python"))) {
    throw new Error(
      "Run bun aseprite:setup first to install the pinned server and Python environment.",
    );
  }
  if (capture(["git", "-C", checkout, "rev-parse", "HEAD"]) !== revision) {
    throw new Error(
      "Aseprite MCP revision differs from the reviewed version. Run bun aseprite:setup.",
    );
  }
  const command = [uv, "--directory", checkout, "run", "--locked", "--no-sync"];
  if (mode === "--check") {
    await checkAseprite(env);
    return;
  }
  const server = Bun.spawn([...command, "-m", "aseprite_mcp"], {
    cwd: root,
    env,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  });
  for (const signal of ["SIGINT", "SIGTERM"] as const) {
    process.on(signal, () => server.kill(signal));
  }
  process.exitCode = await server.exited;
}

try {
  await main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
}
