---
name: effectcraft-cli
description: Use EffectCraft CLI to inspect or edit .ecproj projects, animate layers, render compositions, exchange motion graphics files, run JSX, or drive EffectCraft through MCP or the desktop bridge.
disable-model-invocation: false
---

# EffectCraft CLI

Run `"$HOME/.local/bin/effectcraft-cli"`. Use `--help` for CLI options. EffectCraft exposes editor actions as engine commands with stable IDs and JSON parameters.

1. **Choose the project.** Use `--project <file.ecproj>` for existing work or `--empty` to start fresh. Headless CLI calls open the demo if neither is supplied. Use `info --project <file.ecproj> --json` to identify the active composition, project items, and layers.
2. **Discover commands and targets.** Use `exec --list --filter <term> --schemas --json` with the target project or `--empty`. Check the command ID, schema, and enabled state. If a command needs a different active comp or selection, set that context in the same run. For property paths, use `props <comp> <layer> --flat --json`. Prefer layer IDs when names are ambiguous or layer order may change. Quote selectors such as `'#1'` and JSON arguments.
3. **Edit and persist.** Use `exec <id> --params '<json>'` for one action, `set <comp> <layer> <path> <value>` for a property, or `run --commands <file|->` for ordered steps in a JSON array or JSON Lines. Each headless call starts a fresh engine. Use `--save-as <output.ecproj>` to create a new file or `--save` when updating the input in place. Use one writer per project when mixing CLI and desktop or MCP edits.
4. **Verify the saved result.** Reopen it with `info` or `get` to check the requested state. For visual changes, render representative frames with `render-frame --comp <comp> --time <seconds> --max-side 640 --out <preview.png>` and inspect the images. For a deliverable, use `render --comp <comp> --out <file>` with the requested format and check the generated file or sequence.

Use `--json` to parse stdout and check the exit status. Execution failures exit 1 and include an `error` field in JSON mode. Usage errors exit 2 and can report only to stderr. A failed `run` step reports its zero-based index.

For offset or stretched layers, `set --time` keys in layer seconds. Use `prop.addKey` with `timeBase: "comp"` to key in composition seconds. Check the reported `compTime` for frame snapping. `get --time` and `render-frame --time` sample in composition seconds.

## Headless example

```sh
ec="$HOME/.local/bin/effectcraft-cli"
"$ec" run --empty --save-as main.ecproj --commands - --json <<'JSON'
[
  {"command":"comp.new","params":{"name":"Main","width":320,"height":180,"duration":2}},
  {"command":"layer.newSolid","params":{"name":"Blue","color":"#3366ff"}}
]
JSON
"$ec" props Main Blue --flat --project main.ecproj --json
"$ec" set Main Blue transform/opacity 40 --project main.ecproj --save --json
"$ec" get Main Blue transform/opacity --project main.ecproj --json
"$ec" render-frame --project main.ecproj --comp Main --time 0.5 --out preview.png --json
```

- For desktop control, persistent MCP sessions, or After Effects-style JSX, read [Automation modes](references/automation.md).
- For After Effects projects, Lottie, or timeline exchange, read [File interchange](references/interchange.md).
