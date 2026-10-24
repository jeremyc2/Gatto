# Aseprite MCP

Gatto uses [diivi/aseprite-mcp](https://github.com/diivi/aseprite-mcp) as an optional
local art tool. Its reviewed revision is pinned in
[`scripts/aseprite-mcp.ts`](../../scripts/aseprite-mcp.ts); Python dependencies
come from that revision's `uv.lock`. The server and its environment live in the
ignored `.tools/aseprite-mcp` directory. They are development tools, outside the
app and website builds.

## Setup

Install [Aseprite](https://www.aseprite.org/) and
[`uv`](https://docs.astral.sh/uv/getting-started/installation/). Set `ASEPRITE_PATH`
in the ignored root `.env.aseprite` (see the example below) or your environment,
then run from the repository root:

```sh
bun aseprite:setup
bun aseprite:check
```

Setup downloads the pinned server and lets `uv` install its Python version and
locked dependencies. It can be rerun and preserves a dirty server checkout by
stopping before an update. The launcher finds `uv` on PATH or in `~/.local/bin`.
All Aseprite setup, server, and check commands use `ASEPRITE_PATH` from the process
environment or root `.env.aseprite`. The process variable takes precedence.
Missing configuration produces an error; the launcher has no built-in install path.

To save a machine-specific location, add a literal absolute executable path to
the gitignored root `.env.aseprite` (the binary, not the `.app` folder):

```dotenv
ASEPRITE_PATH="/Applications/Aseprite.app/Contents/MacOS/aseprite"
```

The launcher reads only this setting from `.env.aseprite` and passes the resolved value
to the server for both agent clients and command-line use. An explicitly set
process variable takes precedence, so a one-off override works too:

```sh
ASEPRITE_PATH="/custom/Aseprite.app/Contents/MacOS/aseprite" bun aseprite:check
```

`--no-env-file` disables Bun's automatic dotenv loading; the launcher explicitly
parses `.env.aseprite` as text and selects `ASEPRITE_PATH`. Cloudflare settings
belong in `.env.cloudflare`, which the art launcher never reads. It also strips
inherited `CLOUDFLARE_*` and `CF_*` variables from subprocess environments.
Keep shared root `.env` files free of credentials because an outer `bun` package
command may load them before a nested `--no-env-file` command runs. Upstream can
also read its own checkout's `.env`; keep that file free of unrelated credentials.

The check connects over MCP through the same launcher, verifies instruction and
tool discovery, draws a temporary sprite, checks scaled PNG pixels and alpha,
receives a `view_sprite` image, and exports a two-frame GIF. Temporary artwork is
removed afterward. It requires no API key or running Aseprite window.

The repository-owned check runs in Bun with no additional packages. It uses
[Bun.Image](https://bun.com/docs/runtime/image) for image metadata and decoding,
and Aseprite's existing inspection tools to read exported PNG pixels and verify
the GIF frame count (Bun decodes the first GIF frame). Python stays inside the
ignored upstream server installation.

## Agent clients

- **Codex:** [`.codex/config.toml`](../../.codex/config.toml) defines the project
  stdio server. Project configuration loads for trusted projects. Start a new
  chat after setup to load the server and new skill; `codex mcp get aseprite`
  shows the effective configuration. See [official MCP configuration](https://developers.openai.com/codex/mcp).
- **Claude Code:** [`.mcp.json`](../../.mcp.json) defines the same launcher.
  Allow the project server when Claude Code requests it. Its skill link mirrors
  the repository's existing `.claude/skills` convention.

Open either client at the repository root so its relative launcher path resolves.
Both use [`aseprite`](../../.agents/skills/aseprite/SKILL.md), discoverable for pixel
art work or explicitly with `$aseprite` in Codex and `/aseprite` in Claude Code.
The launcher is also available directly as `bun aseprite:mcp` for stdio clients.

## Paths, maintenance, and troubleshooting

Use absolute asset paths: upstream runs from its own checkout. Create layers
before drawing and destination directories before exporting. See the skill for
frame indexing, palette, drawing, animation, and visual inspection guidance.

- Missing tools: confirm project trust/server enablement, then start a new chat.
- Missing dependencies or Python environment: rerun `bun aseprite:setup`.
- Binary not found: configure `ASEPRITE_PATH` and rerun the check.
- Upgrading: review upstream changes and its workflow instructions, update the
  revision in the launcher, rerun setup and the check, and revise the skill and
  [research record](../research/aseprite-mcp-2026-10-24.md) where guidance changed.

Use `view_sprite` for inspection. Upstream's optional preview HTTP server binds
beyond loopback; only serve a dedicated export folder and stop it when finished.
