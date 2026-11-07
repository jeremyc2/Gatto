# EffectCraft automation modes

## Live desktop

Use `--bridge <port>` when the requested target is the running app. Start or identify an EffectCraft instance launched with `--control <port>`. Run `info --bridge <port> --json` to verify the project and active comp before issuing `exec <id> --params '<json>' --bridge <port> --json`.

If a live mutation loses its reply, inspect the app state before retrying. The engine may already have applied the command.

## Persistent MCP session

Use `mcp --project <file.ecproj>` for repeated tool calls against one headless project, or `mcp --bridge <port>` for a running app. The server speaks JSON-RPC over stdio. Discover the tool schemas with `tools/list`, then use `list_commands` and `describe_command` to inspect engine commands and `execute_command` to run them. For UI work over the bridge, use `ui_elements` to find controls and `screenshot` to check the result.

Headless MCP starts empty unless given `--project` or `--demo`. Use `save_project` before disconnecting or replacing a dirty project. `mcp --autosave` adds recovery checkpoints. Inspect the autosave metadata for checkpoint paths and use `open_project` followed by `save_project` to recover one. The desktop app owns autosaves in bridge mode.

## After Effects-style scripts

Use `script <file.jsx>` or `script --eval '<code>'` for the After Effects-style object model. Pass `--project` for file-based work or `--bridge <port>` for the live app. A headless script starts empty by default. Use `app.run(id, params)` to call engine commands from a script. Inspect `ok`, `error`, and `result` in JSON mode. Group live script edits with `app.beginUndoGroup()` and `app.endUndoGroup()` so partial edits from a script error can be undone together.
