# Aseprite MCP setup and agent guidance

Researched 2026-10-24 against upstream commit
`a89b205bed31930ee141edf319e78c71ef16bba6`. This note records source inspection;
the setup guide and skill own the current local operating instructions.

## Runtime and configuration

- Upstream requires Python 3.13+, `uv`, and an Aseprite executable. Its local
  configuration launches `uv --directory <upstream checkout> run -m aseprite_mcp`;
  the module starts the MCP stdio transport. There is no executable entry point
  declared in the project manifest. [Installation][readme-install],
  [manifest][manifest], [module entry point][main]
- Set `ASEPRITE_PATH` to the actual executable, including the executable inside
  a macOS app bundle rather than the `.app` directory. The command runner passes
  the environment value directly to `subprocess.run`; absent a value it tries
  `aseprite` on PATH. Its dotenv file is resolved beside the **upstream checkout**,
  not the calling Gatto directory. `dotenv` is imported although it is not a
  direct manifest dependency, so retain the tested upstream lockfile/dependency
  resolution. [Command runner][commands], [manifest][manifest]
- Use absolute sprite, import, and output paths. The tools test paths relative
  to their process directory and launch batch commands without a separate working
  directory; `uv --directory` makes that upstream checkout. This is an operational
  inference from the launch configuration and implementations. Create the intended
  output directories first; only some export tools create directories themselves.
  [Installation][readme-install], [command runner][commands], [exports][export]
- Fonts are discovered in `~/.aseprite-mcp/fonts` and system font directories;
  font arguments can also be direct paths. A bitmap font needs `font.json` and its
  sheet images; TrueType/OpenType files can be placed directly in the font
  directory. Bitmap sizes are integer multipliers; TrueType rendering defaults
  to no antialiasing. Use `list_text_fonts` and `measure_text` before drawing
  layout-dependent text. [Text tools and font schema][readme-text],
  [font implementation][fonts]
- Prefer `view_sprite` for inspection. The optional preview server starts
  `python -m http.server` without `--bind`, so the returned `localhost` URL does
  not establish that it is loopback-only. Serve only a dedicated export directory
  and stop the server afterward; use an explicitly loopback-bound alternative
  if needed. [Preview implementation][preview]

## Still-art workflow to carry into the skill

1. Plan the pixel boxes of body parts and props, leaving a 1–2 px margin.
   Typical sprites are 16–64 px; scenes containing a character need at least
   about 128 px. Start with a readable flat silhouette.
2. Pick the palette before color: generate 3–4-step ramps per material, aiming
   for 8–16 colors total, or choose a preset. Separate adjacent materials by
   lightness as well as hue. Keep background contrast and saturation below the
   foreground.
3. Separate background, body, equipment, and effects into layers. Use shapes for
   large masses and `draw_rows` for hand-chosen detail under about 12 px. Straight
   diagonals need regular steps; curves need gradual changes. Remove doubled
   corners, lone pixels, detached parts, and objects hanging off their surfaces.
4. Apply flats, then shade consistently from the top left. Avoid pillow shading;
   skip dithering and antialiasing below 32 px. Keep outlines darker than the fill;
   `outline_cel` grows the shape outward by 1 px.
5. Inspect **every pass** using `view_sprite`, `get_color_stats`, and `lint_sprite`;
   fix silhouette problems before adding detail. Use `region="x,y,w,h"` to inspect
   exact coordinates. Prefer the dedicated drawing, text, recoloring, inspection,
   and export tools before raw Lua. [Server instructions][instructions],
   [recommended LLM workflow][readme-workflow]

The README's general animation step suggests tweens, but the server instructions
and animation guide narrow this: redraw character poses; use cel translation or
tweening for rigid objects and secondary motion. The skill should preserve that
more specific distinction. [Recommended workflow][readme-workflow],
[server instructions][instructions], [animation guide][guide]

## Animation guidance to carry into the skill

- Call `animation_workflow_guide` **before animating**, choosing `character`,
  `idle`, or `environment` as appropriate.
- Character motion starts with key poses: anticipation, action, follow-through.
  Copy a frame, erase the moving part, and redraw it; drawing over the previous
  limb leaves ghosts. Involve the whole body, retain part dimensions/volume, and
  keep feet on one ground line except during airborne motion. Attacks usually
  use 4–6 frames, jumps 5–7, walks 6–8. Soft bodies deform around 30–50% and change
  shape on every airborne frame.
- Vary timing: normal poses about 80–120 ms, anticipation holds 150–250 ms, and
  strikes or a single smear frame 50–80 ms. Preserve the palette across frames.
- Idle loops use 2–6 frames at 150–300 ms. Torso, shoulders, head, arms, and weapon
  rise/fall together by about 1 px; hair and cloth can lag. Feet stay fixed and
  knees soften as the hips drop. Avoid a wide, deeply bent stance that reads as
  dancing. `pingpong` tags suit a short breathing loop.
- Environments share one palette and a copied base scene. Propagate static sky,
  mountains, and ground; animate moving layers such as clouds, birds, and water.
  Copy reusable layers between sprite files.
- Verify with `render_onion_skin` (the character guide suggests scale 6),
  `compare_frames` for an isolated part/region, `get_bounds`, `get_color_stats`,
  and `audit_animation`; use `animation_sanitize` where appropriate. Loops must
  close into the first frame. Inspect frame sheets with `view_sprite` because
  an animated GIF cannot provide frame-by-frame model inspection; export tagged,
  scaled GIFs or PNG sequences as deliverables. [Animation guide][guide]

## Tool pitfalls

| Concern | Guidance and evidence |
| --- | --- |
| Indexing | MCP frames start at **1**; pixel coordinates start at **0**. Frame-range ends are inclusive. Dedicated export tools translate to the CLI's zero-based `--frame-range`; do not pre-subtract. [Drawing][drawing], [animation][animation], [export][export] |
| Active state | Every operation launches a separate Aseprite batch process. Pass explicit layer/frame arguments using `_at` tools or `draw_rows`, rather than relying on a previous `set_layer` or `set_frame` call. This recommendation follows from the process boundary. [Command runner][commands], [canvas][canvas], [drawing][drawing] |
| Layer creation and errors | Create named layers before drawing: `create_if_missing` creates a cel on an existing layer. Tools can return failure messages as ordinary strings rather than MCP errors; inspect the result and verify the changed pixels. Confirmed during the local MCP smoke check. [Drawing][drawing], [shared cel helpers][lua] |
| Cels and offsets | Coordinates are sprite-global. `draw_rows` expands/normalizes existing cels to the canvas and clips off-canvas pixels. Some older `_at` implementations, including `draw_pixels_at`, only subtract the cel offset and write into its existing image: keep coordinates inside that image or choose `draw_rows` for extending trimmed/moved cels. [Drawing][drawing], [shared cel helpers][lua] |
| Row edits | Equal-width rows are required. `.` and unmapped characters leave old pixels untouched; map a character to `#00000000` or use `erase_region` to remove pixels. [Drawing][drawing], [selection][selection] |
| Alpha | Colors support `#RGB`, `#RGBA`, `#RRGGBB`, and `#RRGGBBAA`; omitted alpha is 255. Alpha is last, not first. Some RGB-specific operations intentionally discard alpha, so consult the particular tool. [Color parser][colors], [selection][selection] |
| Inspection | `view_sprite` returns MCP image content with nearest-neighbor scaling, optional grid/rulers, and up to 16 selected frames (`"1-4"`, `"1,3,6-8"`); an empty selector shows at most the first 16. [View implementation][view] |
| Lua | Use `run_lua_script` only for operations dedicated tools cannot express. Save with `spr:saveAs(spr.filename)` and print results; the script runs unrestricted host code. [Scripting guidance][readme-script] |

[readme-install]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/README.md#local-installation
[readme-text]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/README.md#text
[readme-workflow]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/README.md#recommended-workflow-for-llms
[readme-script]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/README.md#scripting
[manifest]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/pyproject.toml
[main]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/__main__.py
[instructions]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/__init__.py
[commands]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/core/commands.py
[fonts]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/core/fonts.py
[colors]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/core/colors.py
[lua]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/core/lua.py
[guide]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/guide.py
[drawing]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/drawing.py
[animation]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/animation.py
[canvas]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/canvas.py
[selection]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/selection.py
[preview]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/preview.py
[view]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/view.py
[export]: https://github.com/diivi/aseprite-mcp/blob/a89b205bed31930ee141edf319e78c71ef16bba6/aseprite_mcp/tools/export.py
