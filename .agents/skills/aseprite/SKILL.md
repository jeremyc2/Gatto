---
name: aseprite
description: Create, edit, inspect, and export pixel art or sprite animations with Aseprite MCP. Use for .aseprite assets, pixel sprites, tiles, palettes, and frame animation.
---

# Aseprite

Use the `aseprite` MCP tools for editable pixel art. The repository launcher pins
the reviewed upstream server. For a missing server or a new checkout, follow
[setup and troubleshooting](../../../docs/agents/aseprite.md).

## Working files

- Keep editable `.aseprite` sources alongside the relevant artwork records and
  export the formats the caller needs. Preserve approved artwork unless a change
  was requested; create a separate working copy when exploring a variant.
- Create destination directories first. Pass **absolute paths** for sprites,
  imports, and exports: the server runs from its own checkout, not Gatto.
- Inspect existing work with `get_sprite_info` and `view_sprite` before editing.
  Use explicit layer and frame arguments (`*_at`, `draw_rows`); each call opens
  a separate batch process. Create layers with `add_layer` before drawing;
  `create_if_missing` on drawing tools creates a cel, not a layer.
- Read tool result text: upstream can report a failed operation as a normal
  string without setting MCP's error flag. Verify the changed pixels or file.
- Pixel coordinates start at **0**; MCP frames start at **1**, with inclusive
  frame ranges. Use `#RRGGBBAA` for alpha. `draw_rows` requires equal-width rows;
  `.` and unmapped characters preserve pixels. Erase with `erase_region` or a
  character mapped to `#00000000`. Prefer `draw_rows` when extending a trimmed cel.

## Draw in passes

1. Plan the pixel boxes of parts and props; leave a 1–2 px margin. Start at the
   intended native resolution: typically 16–64 px for sprites, about 128 px or
   larger for a scene with a character.
2. Plan one palette: `generate_color_ramp` with 3–4 steps per material, aiming
   for 8–16 colors total, or `apply_palette_preset` for an established retro look.
   Separate neighboring materials by lightness as well as hue.
3. Split background, body, equipment, and effects into layers. Block a readable
   flat silhouette with shape tools, then refine with `draw_rows` / `read_rows`.
   Draw faces, hands, animals, and handles under about 12 px pixel by pixel.
   Keep diagonal steps regular and curve steps gradual; remove doubled corners
   on thin lines. Attach parts and place objects fully on their surfaces.
4. Apply flats, then highlights and shadows from one light direction (upstream
   defaults to top left), including shadows under overhangs. Avoid pillow shading.
   Below 32 px, use clean clusters without dithering or antialiasing. For larger
   art, use `adjust_hsl`, `apply_dither_gradient`, and `outline_cel` deliberately.
   Keep outlines darker than adjacent fills; outlines grow 1 px outward.
5. After **every pass**, inspect `view_sprite`, `get_color_stats`, and
   `lint_sprite`; fix stray colors, disconnected parts, and lonely pixels before
   proceeding. Use `region="x,y,w,h"` to fix exact pixels. If the silhouette
   fails to read, reshape it before adding detail.

## Animate and deliver

Before animating, call `animation_workflow_guide` with `use_case="character"`,
`"idle"`, or `"environment"`. Read [animation guidance](references/animation.md)
for the corresponding branch. Redraw main character poses; reserve cel tweens
and offsets for rigid objects and secondary motion.

Inspect individual frames with `view_sprite` (selectors such as `"1-4"` or
`"1,3,6-8"`, up to 16 frames per call), plus `render_onion_skin`, `compare_frames`,
and `get_bounds`. Check palette consistency and run `audit_animation`; use
`validate_scene` / `animation_sanitize` when layer coverage needs checking or
repair. Verify that loop endpoints flow together.

Export with `export_frame`, `export_animation`, `export_tag`, or
`export_spritesheet` (including JSON metadata when needed). Use integer
nearest-neighbor scaling; optional preview backgrounds belong on exports, not
the editable source. Deliver the source and relevant exports, describing native
size, frame timing, tags, and export scale. Inspect frame sheets to judge drawing;
GIFs serve as playback deliverables.

## Specialized tools

- Text: use `list_text_fonts` to choose an available bitmap or system font, or
  pass a font path. Call `measure_text` before layout-dependent `draw_text`.
  Bitmap sizes scale by whole numbers; fonts can live in `~/.aseprite-mcp/fonts`.
- Recoloring: prefer `replace_color` / `remap_colors_in_cel_range`; keep animation
  frames on the intended palette with `set_palette` and `quantize_to_palette`.
- Preview: prefer `view_sprite`. Upstream's optional HTTP preview listens on all
  interfaces; serve only a dedicated export folder and stop it afterward.
- Lua: use `run_lua_script` only where dedicated tools cannot express the work.
  Read the script before running it: it has unrestricted host access. Save with
  `spr:saveAs(spr.filename)` and `print()` results; batch Lua return values vanish.

Guidance comes from the upstream [recommended workflow](https://github.com/diivi/aseprite-mcp#recommended-workflow-for-llms),
connection instructions, and animation guide. See the commit-pinned
[research record](../../../docs/research/aseprite-mcp-2026-10-24.md) for source
citations and implementation caveats when maintaining this skill or upgrading.
