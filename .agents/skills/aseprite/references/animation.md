# Animation branches

Call `animation_workflow_guide` for the relevant branch before drawing. These are
the reviewed upstream defaults; adapt timing and frame counts to the art direction.

## Character

- Plan anticipation, action, and follow-through: attacks typically take 4–6
  frames, jumps 5–7, and walks 6–8. Copy a frame, erase the changing part, then
  redraw it. Involve the whole body: weight shift, torso turn, and following head.
- Keep part dimensions consistent and feet on a shared ground line unless
  airborne. Maintain volume through squash and stretch. Soft bodies deform by
  roughly 30–50% and change shape on every airborne frame.
- Start with 80–120 ms normal poses, 150–250 ms anticipation holds, and 50–80 ms
  strikes. A fast action can use one 50–80 ms smear along the motion arc.
- Use onion skins (upstream suggests scale 6), bounds, and regional frame diffs
  to check attachment and drift. Zero changed pixels means a supposedly moving
  part stayed still; a whole-sprite change on a small action suggests drift.
- Keep the same palette across frames; investigate changes in color statistics
  before quantizing. Tag action ranges, close loops, and audit layer coverage.

## Idle

- Use a 2–6-frame breathing loop at about 150–300 ms per frame. Attacks start
  from and return to this pose.
- Keep a relaxed stance with nearly straight knees. Torso, shoulders, head,
  arms, and held weapon rise/fall together by about 1 px; hair, cloth, or a weapon
  tip may lag a frame. Knees soften forward as hips drop; feet remain fixed.
- Check the feet with regional `compare_frames`, and head height with
  `get_bounds`. A 2–3-frame breath often suits a `pingpong` tag. Ensure the loop
  closes without a jump.

## Environment

- Share one palette; keep backgrounds lower in contrast and saturation than
  foreground subjects. Build the base scene once and use `copy_sprite` for variants.
- Use `propagate_cels` for static sky, ground, and mountains. Tween or offset
  moving layers such as clouds, birds, or water; use
  `copy_layers_between_sprites` for reusable pieces.
- Inspect loop endpoints with onion skins and frame views. Audit missing cels,
  layer activity, and overlaps before exporting tagged loops for pacing review.
