# AI-assisted assets for a sophisticated 2D indie game

Research date: October 18, 2026 (America/Chicago).

This note investigates a production strategy, suitable agent skills, and the practical limits of ChatGPT-style image generation for characters, animation, and exploratory worlds. It does not select a game camera, approve an art style, install tools, or generate assets. Product capabilities are a dated snapshot; vendor examples are not comparative benchmarks.

## Recommendation

Use AI for **art exploration, reference-guided variations, and selected production assistance**, while keeping character construction, motion, world assembly, and export under explicit control. Approve a small set of master assets, then derive the game from those. Judge the result in motion at the actual gameplay size.

For a side-view game, begin with deliberately simple drawn characters or a layered 2D rig, depending on the desired motion. For a four-direction game, test a manually corrected directional model sheet and a small animation set. For eight or more directions, complex actions, frequent costume changes, or a freely rotating camera, seriously test **rigged 3D rendered as 2D** before committing to independently generated sprite frames. These are production recommendations inferred from the evidence below, not a universal ranking of tools.

The best-matched existing agent skill found is OpenAI's **Game Studio `sprite-pipeline`**, used with `imagegen`. It provides an unusually concrete starting point: an approved game frame, one animation strip per generation, normalization, and an engine preview. Its scope is narrower than a complete multidirectional character pipeline. [Pinned skill source](https://github.com/openai/plugins/blob/d965fdc57d1f872f52bcdb10c9e1cbf922a8a761/plugins/game-studio/skills/sprite-pipeline/SKILL.md)

The central distinction is **an attractive sheet is evidence of a visual idea; a production sprite set is a tested system**. A beautiful turnaround can still contain contradictory anatomy. A tidy animation grid can still slide, resize, change costume, or omit a recovery pose.

If “skill” means the human capability to develop, prioritize **art direction and animation judgment**: selecting shapes, understanding volume, staging poses, timing contacts, and correcting a family of assets. Prompt writing helps express those decisions; it cannot replace deciding whether the character and motion are right. This is our synthesis, rather than a measured ranking of artistic skills.

## What the primary sources establish

### Generative imagery offers useful references, not guaranteed identity or geometry

OpenAI documents image inputs, editing, iterative workflows, and transparent output. Its current image guide also explicitly acknowledges recurring-character consistency and precise composition limitations. Masks guide an edit rather than guaranteeing an exact pixel boundary. Those are direct reasons to verify every direction and frame rather than treating a successful prompt as a production guarantee. API settings should not be assumed to exist in the ChatGPT interface or Codex's built-in image tool. [OpenAI image-generation guide](https://developers.openai.com/api/docs/guides/image-generation)

In ChatGPT, attach the approved image when requesting an edit or using visual guidance. For multiple references, identify each image's role: character identity, style, camera, or layout. Make small revisions and repeat what must remain fixed. These practices follow the official image-generation guidance; they provide a practical art-direction workflow without establishing animation correctness. Availability in ChatGPT web depends on plan and workspace settings. [ChatGPT image-generation guidance](https://learn.chatgpt.com/docs/image-generation)

These different controls address different problems:

| Control                                         | Supported purpose                                                                                     | Production implication — our synthesis                                                                                                            |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reference images / image conditioning           | IP-Adapter combines image prompts with text and can work alongside structural controls.               | Useful for appearance; separately inspect identity landmarks and pose.                                                                            |
| Pose, edge, depth, or segmentation conditioning | ControlNet's original work demonstrates spatial conditioning of diffusion models using these signals. | Supply designed geometry rather than expecting text to specify all joints and contacts.                                                           |
| Character/style training                        | Scenario documents character models and recommends trying references before training.                 | Consider training after a repeated need is demonstrated, using an approved dataset; measure drift rather than believing a promise of consistency. |
| Multi-view generation                           | Zero123++ targets consistent views from a single image.                                               | Useful as a reconstruction/reference experiment; not proof of correct unseen anatomy or a finished animated rig.                                  |
| Sprite-specific research                        | Sprite Sheet Diffusion investigates generating character animation sheets.                            | Relevant experimental lead, not evidence that a shipping pipeline can omit cleanup, metadata, or engine tests.                                    |

Sources: [IP-Adapter paper](https://arxiv.org/abs/2308.06721), [ControlNet paper](https://arxiv.org/abs/2302.05543), [Scenario character-model guide](https://help.scenario.com/articles/4280773511-train-a-consistent-character-model), [Zero123++ paper](https://arxiv.org/abs/2310.15110), [Sprite Sheet Diffusion paper](https://arxiv.org/abs/2412.03685).

The research papers concern particular architectures and experiments. They are not ChatGPT features, and their published examples do not establish which current commercial service will perform best for this game's style. Compatibility, setup effort, hardware requirements, and licenses need checking for any implementation chosen.

### Distinctive shipped art comes from production decisions

Three first-party examples illustrate different choices rather than one correct aesthetic:

- **Hollow Knight:** Ari Gibson describes a simple, near-monotone style and traditional 2D animation, explicitly connecting reduced detail to efficient creation of varied enemies and environments. The transferable lesson is to choose repeatable shapes and a manageable detail budget. [Team Cherry's introduction](https://www.teamcherry.com.au/blog/introducing-hollow-knight)
- **Dead Cells:** artist Thomas Vasseur describes a pixel model sheet, basic 3D model and skeleton, low-resolution rendering, and exported PNG frames and normal maps. Key poses and timing come first. He emphasizes retakes and reuse, while acknowledging flickering pixels and detail compromises. This demonstrates a real hybrid pipeline, including its costs. [Vasseur's production account](https://www.gamedeveloper.com/production/art-design-deep-dive-using-a-3d-pipeline-for-2d-animation-in-i-dead-cells-i-)
- **Cuphead:** Studio MDHR identifies traditional cel animation and watercolor backgrounds as actual production methods. Background artist Caitlin Russell describes multiple layers for movement and parallax. The transferable lesson is to design the medium and scene assembly together, not merely request a watercolor-looking screenshot. [Studio MDHR press kit](https://studiomdhr.com/press-kits/), [Russell's background portfolio](https://caitrussell.com/ch-bgs.html)

These are evidence for those projects' processes. They do not imply their scale, staffing, or quality can be reproduced by a solo developer through prompting.

## Pick the camera and art process together

The following is a decision framework, not a measured price or quality comparison.

| Visual direction and camera                     | Suggested starting process                                                                       | Useful AI work                                                                | Principal risk to test                                                                          |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Low-resolution pixel art; side view             | Aseprite master sprites and hand-corrected key poses                                             | Silhouettes, palette ideas, rough props, pose suggestions                     | Uneven pixel clusters, drifting pixel scale, noisy shading, time spent repainting               |
| Drawn/cel style; side view                      | Designed key poses; frame animation or a hybrid with selected rigged parts                       | Character exploration and rough pose variations                               | Generic shapes, rubbery rig motion, altered line weight and proportions                         |
| Paper/collage; side or limited quarter view     | Layered pieces, explicit overlaps, pivots and material samples                                   | Source textures, shape studies, alternative construction                      | Texture swimming, impossible joints, incoherent cast shadows                                    |
| Painterly/watercolor; exploration game          | Designed silhouettes, stable value groups, layered room paintings and a restrained character set | Color keys, architecture studies, material exploration, background paintovers | Brush texture changing between frames; beauty at full resolution but poor game-scale legibility |
| Four-direction top-down                         | Corrected cardinal views, then a small action library per view                                   | Directional proposals based on approved front/side/back references            | Front/back contradictions, unseen limbs, excessive cleanup                                      |
| Eight/sixteen-direction isometric               | Compare 3D-to-2D against hand-drawn directional sets                                             | Model/texture concepts, reference cleanup, supporting props                   | Rigging cost versus cumulative per-frame/per-angle repair                                       |
| Freely rotating camera; many equipment variants | Rigged 3D with controlled rendering, possibly runtime 3D in an otherwise 2D presentation         | Concepts and texture drafts                                                   | A sprite-angle inventory becoming too large; preserving a deliberate illustrated finish         |

Aseprite supports tagged frame selection and sprite-sheet export; Spine supports mesh deformation and image/data export; neither automatically establishes a character's hidden geometry. [Aseprite sprite sheets](https://www.aseprite.org/docs/sprite-sheet/), [Spine mesh attachments](https://esotericsoftware.com/spine-meshes), [Spine export](https://esotericsoftware.com/spine-export)

**Suggested default for a small exploratory game:** use a deliberately authored, relatively simple character over richer, layered environments. Limit facing directions and actions until a pilot demonstrates more are affordable. The sophistication can come from shape, mood, composition, animation timing, and a specific world rather than minute detail in every sprite.

## Separate the deliverables

Use these working definitions so an agent cannot accidentally substitute one for another:

| Deliverable                          | What it answers                                                       | Acceptance criterion                                                                       |
| ------------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Concept sheet                        | Which shapes, costume, personality, and materials are worth pursuing? | A direction is chosen; alternatives may disagree.                                          |
| Canonical model/turnaround sheet     | What is the same character from each required view?                   | Proportions and asymmetric features agree; camera is specified.                            |
| Pose and expression library          | How does the character act?                                           | Readable gestures and emotional range without redesigning the body.                        |
| Animation source                     | How does a particular action unfold?                                  | Stable identity, authored contacts and timing, editable frames or rig.                     |
| Runtime sprite sheet / texture atlas | How does the engine locate and play approved imagery?                 | Valid pixels plus frame rectangles, order, durations, offsets and other required metadata. |
| World kit                            | How do places combine into navigable spaces?                          | Repeatable modules, transitions, layers, landmarks, and tested gameplay readability.       |

Aseprite distinguishes sprite sheets and texture atlases, and Godot demonstrates importing either separate frames or a sheet into an animation resource. The exact format depends on the chosen engine, so validate one import early. [Aseprite documentation](https://www.aseprite.org/docs/sprite-sheet/), [Godot 2D sprite animation](https://docs.godotengine.org/en/stable/tutorials/2d/2d_sprite_animation.html)

### Budget the combinatorics before rendering

For a frame-based character, calculate:

```text
unique frames = sum over actions(required directions × frames per direction)
```

An illustrative scope is 8 directions × (4 idle + 8 walk + 6 interact + 4 hurt) = **176 frames** for one character before costume variants. Three completely distinct costumes could require **528 frame images**. This is arithmetic for a hypothetical scope, not a suggested standard. Shared frames, held poses, mirroring, rigs, and separate equipment layers can change the count.

Do not confuse animation frames per second with the game's render rate. Record durations per frame and design held poses. A loop should not need a duplicated first frame at the end unless the timing intentionally includes that hold.

**Mirroring is a design decision.** A horizontally flipped view can be acceptable for a sufficiently symmetric character. It reverses handedness, lettering, scars, bag straps, weapon sides and baked lighting. For asymmetric designs, require true opposing views or a deliberate simplification. With eight directions, five unique view families can cover the circle by mirroring three pairs only when the design and lighting allow it. Turning a flat image in its plane does not create a new camera view.

For an animal, lock muzzle depth, ear shape, shoulder/hip placement, limb count and tail attachment across front, rear and quarter views. Ask specifically what the far legs do. For equipment, decide whether the item is embedded in each frame or attached independently; the latter needs consistent sockets and occlusion rules.

Describe anatomical left/right separately from screen left/right. Define whether illumination is fixed to the camera/world or rotates with the asset. Otherwise a correctly turned body can still acquire a swapped scar or contradictory highlight.

## A repeatable production pipeline

Everything in this section is a proposed workflow derived from the source capabilities and limitations, not a vendor guarantee.

### 1. Make an art bible and technical contract

Keep the contract small enough to use during every review:

- **Art intent:** three descriptive principles, one clear world premise, and explicit examples of what should feel unusual or specific.
- **Shape rules:** silhouettes, head/body relationships, negative spaces, line character, and allowed exaggeration.
- **Identity:** named landmarks and asymmetries, a corrected front/side/back sheet, expressions, and an approved neutral game frame.
- **Material:** actual paper/paint/ink samples or authored digital equivalents; where texture belongs and how it moves.
- **Color/value:** palette roles, light direction, contrast hierarchy, background versus interactable treatment. Treat prompted color codes as a request; inspect output values.
- **Camera:** projection, elevation, azimuths for each direction, visible surfaces, ground plane, and sprite scale.
- **Motion:** action list, pose keys, contact sequence, frame durations, transitions, and acceptable deformation.
- **Runtime:** logical canvas, root/pivot, source dimensions, alpha treatment, frame naming, atlas metadata, filtering, padding and texture limits.
- **World:** grid or module sizes, collision surfaces, exits, depth layers, landmark density and occlusion rules.
- **Provenance:** reference ownership, source files, model/tool version, prompt and reference roles, selected output and manual edits.

Give agents a fixed reference packet: identity sheet, style samples, actual gameplay screenshot, camera guide and target frame layout. Label the role of each image; otherwise a style sample can inadvertently become a new costume or composition.

### 2. Explore broadly, then approve narrowly

Generate deliberately different silhouettes and material approaches. Compare them at the target game size on light and dark backgrounds. Pick a direction, correct it, and freeze a master. Keep rejected experiments separate from approved references.

Use a new generation for genuinely different ideas and targeted edits for local changes. Return to the approved master when drift accumulates. Do not make a long chain of slightly changed outputs the only source of identity.

Repair a rejected frame or region while preserving accepted source frames, then repack the atlas. Regenerating the complete accepted sheet to fix one defect creates another round of identity and motion review.

### 3. Resolve the turnaround before animation

Create the required directional views in a neutral pose with matched projection, scale and ground plane. Compare anatomical landmarks, costume construction and object attachment across them. Draw corrections over the proposals. A plausible back view is invented information until it is deliberately approved.

For a true rotation-heavy character, build or commission a simple 3D proxy, approve its proportions, rig it, and render the required views from fixed cameras. Apply the intended painting/line/texture treatment consistently, then correct selected silhouettes. The point of the proxy is dependable volume and placement; a final pixel or painted pass can still be necessary.

### 4. Author motion before surface detail

Define anticipation, contact, passing, impact and recovery poses for the action as relevant. Test their timing as a rough silhouette. For a walk, specify an in-place cycle versus deliberate root motion, stride length, and foot contact sequence. For a quadruped, specify the gait rather than simply requesting “walking.”

For AI strips, generate one short action in one direction from its approved seed in a single request. A shared strip is a useful consistency tactic; split actions and views into manageable batches. Specify exact slots and pose intent, then verify the output contains them. Do not independently regenerate every finished frame with only the same text description.

For layered rigs, draw obscured material behind joints and plan attachment swaps for turns and perspective changes. Mesh deformation is useful, but a profile drawing stretched around bones is not automatically a convincing back view. For a hybrid, use the rig for body motion and authored replacement poses for dramatic foreshortening, hands, face, or impact shapes.

### 5. Normalize without erasing the performance

Keep one **character-wide scale** across every direction and action. Align by a designed root/ground landmark or rig origin, not the changing outer silhouette. Preserve jumps, leans and attack reach relative to that origin. Never independently resize each frame to fill its box.

If trimming transparent borders, store the offset and original logical dimensions so the engine reconstructs the intended placement. Aseprite's slices can hold pivots and export slice data to JSON; its CLI can export textures plus JSON, tags, slices and layers. [Aseprite slices](https://www.aseprite.org/docs/slices/), [Aseprite CLI](https://www.aseprite.org/docs/cli/)

Pack only approved frames. Keep deterministic source ordering, stable identifiers, durations, offsets, and documented padding/extrusion settings. Aseprite exposes trimming, padding and extrusion controls in its sprite-sheet export API. Match the export to the runtime importer. [Aseprite ExportSpriteSheet API](https://www.aseprite.org/api/command/ExportSpriteSheet)

Repair a rejected frame or local region from the approved sources, then repack. Regenerating an entire accepted sheet to fix one paw exposes every already-approved frame to another change.

### 6. Review in the game before scaling the library

Watch loops and action transitions at normal speed and in slow motion. Test every direction while moving through a real room, including behind foreground objects. Inspect hit/contact moments against the game's actual collision and interaction logic. Keep visual effects and contact shadows separate when that improves reuse or occlusion control.

Choose filtering for the medium. Godot's CanvasItem documentation distinguishes nearest filtering from linear/mipmap alternatives; it specifically recommends mipmapped filtering for non-pixel art viewed at small scales. Do not copy pixel-art export assumptions into watercolor sprites. [Godot CanvasItem texture-filter documentation](https://docs.godotengine.org/en/stable/classes/class_canvasitem.html#enum-canvasitem-texturefilter)

Suggested acceptance gates for each delivery:

- **Coverage:** every required action/direction exists, frame counts match the manifest, and permitted mirrored views are explicit.
- **Image integrity:** dimensions match the contract; real alpha survives export; no baked checkerboard, clipped extremities, stray background pixels or unwanted edge halos. Inspect against both light and dark backgrounds.
- **Identity and motion:** landmarks, material and scale survive every view and state transition; contacts, root offsets and deliberate deformations read correctly.
- **Data integrity:** frame identifiers are unique, rectangles fit inside the atlas, durations are valid, and trim offsets restore the original logical placement. Keep gameplay events and collision shapes explicitly authored.
- **Runtime:** the imported result passes playback, occlusion, texture-edge and memory-budget review in the target game.

Structural checks can reject malformed deliveries automatically. Visual acceptance still needs animation playback and art judgment.

## The strongest skill starting point, and necessary extensions

The essential human skills are art direction, character construction, pose design and animation timing. An agent skill can preserve decisions and automate delivery; someone still needs to judge whether the character has weight, appeal and a coherent place in the world.

Use the pinned **`sprite-pipeline`** as a starting specification, paired with the installed **`imagegen`** for actual raster generation. The upstream skill explicitly requires an approved seed, whole-strip generation, shared strip scale and anchor, optional first-frame lock, and engine review. It is a better match than a generic prompt collection. This is source inspection, not an installation or quality benchmark. [OpenAI Game Studio skill](https://github.com/openai/plugins/blob/d965fdc57d1f872f52bcdb10c9e1cbf922a8a761/plugins/game-studio/skills/sprite-pipeline/SKILL.md)

**Important implementation caveat:** the supplied normalizer crops each slot to its alpha bounds, recenters it horizontally, and bottom-aligns it; it chooses a shared scale for the current strip and uses nearest-neighbor resizing. Our inference is that this can erase intended jumps or horizontal offsets, make a long attack shrink a whole strip relative to another action, and mishandle a painted finish. It cannot repair anatomy or identity drift. [Inspected normalizer](https://github.com/openai/plugins/blob/d965fdc57d1f872f52bcdb10c9e1cbf922a8a761/plugins/game-studio/scripts/normalize_sprite_strip.py)

Before using it for a substantial game, extend or wrap its process with:

1. A character-wide master scale, logical canvas, and root landmark contract.
2. Per-frame offsets preserved through trimming; action-specific rules for planted feet versus airborne/root-moving frames.
3. A states × directions coverage manifest and explicit allowed mirror relationships.
4. Native editable sources, duration/event metadata, stable export identifiers and importer verification.
5. Style-specific resampling and alpha rules, including soft watercolor edges.
6. Automated structural checks followed by human visual review; normalization must never be mistaken for acceptance.

The local `imagegen` skill handles generation/edit execution and reference-directed iteration; it does not itself define engine atlases or multidirectional animation acceptance. See the [installed imagegen skill](/Users/jeremy/.codex/skills/.system/imagegen/SKILL.md). No skill was installed or changed for this research.

For **pixel art**, include PixelLab in the first specialist pilot against ChatGPT. Its current API lists eight-direction generation from a south-facing reference, text-guided animation from a start/optional end image, and a beta skeleton-guided route with a skeleton per frame. The listed v3 limits are 256 × 256 pixels, with up to 16 frames for text animation and 3–15 for skeleton animation. These are vendor-documented capabilities, not independently verified consistency or quality. Its older endpoints have different limits; do not generalize those to every tool. [PixelLab API model listing](https://www.pixellab.ai/pixellab-api)

For larger asset libraries, Scenario is worth a **pilot**, not an automatic purchase recommendation. Its own guidance says to try image references before character training and documents reusable generation/editing workflows. Its spritesheet tutorial actually generates video, manually extracts key poses, and packs them; it acknowledges imperfect motion. That may be useful for motion ideas or ambient loops. Require fixed camera, coherent anatomy, clean extraction, alpha, contact timing and loop tests before using it for a player character. [Scenario character models](https://help.scenario.com/articles/4280773511-train-a-consistent-character-model), [Scenario creative workflows](https://help.scenario.com/articles/7094354401-building-workflows-in-scenario), [Scenario spritesheet tutorial](https://help.scenario.com/articles/9088582240-create-spritesheets-with-scenario)

## Build worlds as reusable places

Generate world **ideas** more freely than production geometry. For an exploration game, start with a map and playable blockout: paths, room dimensions, exits, hiding places, sightlines and landmarks. Then establish biome color keys and a modular kit that can dress that layout.

A suggested kit for one small biome:

| Family          | Examples                                                 | Contract to preserve                                            |
| --------------- | -------------------------------------------------------- | --------------------------------------------------------------- |
| Structural      | Ground, walls, edges, corners, bridges, steps            | Traversal size, repeat edges, perspective and collision surface |
| Reusable props  | Doors, lamps, furniture, plants, signs                   | Scale, grounding, pivots, interaction state                     |
| Landmarks       | Distinctive tree, ruin, machine, gathering place         | Story role, silhouette and orientation cues                     |
| Dressing        | Debris, moss, cracks, paper, small stones                | Controlled variation; avoid masking paths and cues              |
| Depth           | Distant silhouettes, middle ground, foreground occluders | Parallax, layer boundaries and player readability               |
| Animated pieces | Water, foliage, curtains, ambient creatures              | Loop timing, stable material and attachment                     |
| Effects         | Light shafts, fog, particles, weather                    | Contrast budget, performance and occlusion                      |

For tile-based spaces, author transition coverage explicitly. Tiled's terrain sets distinguish corner-, edge-, and mixed matching; Godot's TileSet documentation covers terrain transitions, collisions, metadata and sorting origins. A seamless texture alone does not supply that information. [Tiled terrains](https://doc.mapeditor.org/en/stable/manual/terrain/), [Godot TileSets](https://docs.godotengine.org/en/stable/tutorials/2d/using_tilesets.html)

Suggested workflow: approve one room, extract its material/shape vocabulary, generate individual kit proposals, clean and assemble them in the editor, then build several different rooms from the same kit. Add bespoke landmarks where they help orientation. Test unusual junctions, indoor/outdoor transitions, foreground passage, repeated modules and every required door state.

For painted environments, retain separate backdrops, occluders, foreground and interactive props. Request a beautiful complete scene for a color key; produce actual layers from the authored room plan. Do not assume an image generator has produced editable depth merely because the result looks spatial.

For worlds with different media, keep common camera, scale, contact and interaction rules while allowing each world's texture, palette and construction to differ. Keep the protagonist's identity landmarks stable. Treat each new world as another small kit with a style sheet rather than a fresh global prompt for every room.

## Avoiding a generic AI finish

There is no objective “AI-looking” detector in this research, and these are art-direction recommendations rather than empirical claims about every generated image.

- **Describe construction, not prestige adjectives.** Specify flat ink shapes, visible paper overlaps, sparse dry-brush marks, a limited palette or carefully designed pixel clusters. “Masterpiece, cinematic, ultra-detailed” does not define a repeatable medium.
- **Create a particular visual world.** Use authored architecture, locally meaningful objects, recurring motifs, purposeful wear and character gestures. Avoid selecting every attractive variation; maintain a small shared visual grammar.
- **Use detail selectively.** Preserve resting areas and big value masses. Spend detail where the player reads identity, action or a landmark.
- **Keep material behavior coherent.** Grain should remain attached to a surface; an ink contour should not become an airbrushed edge in the next pose. Decide whether shadow and highlight are painted into sprites or produced in the game.
- **Curate and repaint.** Correct silhouettes, anatomy, joins, hands/paws, edge quality and visual rhythm. Remove invented ornaments that do not survive other angles. A consistent reduction can improve the whole library more than another upscale.
- **Use human-authored anchors.** A commissioned model sheet, a small painted prop kit, photographed original materials, or a designer's corrected key poses can provide the vocabulary for many assisted assets.
- **Review the work as a set.** Put the protagonist, one enemy, a door, a small prop and a room together. Individually good images can still belong to incompatible games.

The aim is to make the work feel intentionally art-directed. Preserve the source and edit history so another artist or agent can reproduce accepted decisions.

## Copyable prompt recipes

These templates are proposed starting points. Replace bracketed values, attach the named references, and inspect results. Exact layout, identity, colors and alpha remain verification tasks; a prompt is not a test result.

### A. Explore a style without accidentally approving a sprite

```text
Create a concept study for [character/world premise].
Explore [three clearly different silhouette/material directions].
The game camera is [side/top-down/isometric, elevation].
The character must read at approximately [height] screen pixels.
Prioritize [shape principle], [palette role], and [personality/gesture].
Reference A sets material only; reference B sets camera only.
Do not copy their characters, costume, or composition.
Show [count] distinct proposals on a plain background.
This is concept exploration, not an animation or final runtime sheet.
```

### B. Establish one directional master

```text
Use reference A as the approved character identity and reference B as
style/material guidance. Produce one full-body [rear three-quarter]
neutral view at the same camera elevation and scale as reference A.

Preserve: [head/body ratio], [ear/muzzle shape], [costume construction],
[tail attachment], [asymmetric feature and its anatomical side].
Camera: [projection, elevation, facing azimuth].
Ground contact and framing: [ground line/root position], full silhouette
visible with room for [tail/weapon].
Material: [specific edge/mark-making rules], [palette].
Transparent background; no floor, cast scenery, labels, or effects.
Do not redesign hidden parts arbitrarily: follow the attached back/side guide.
```

### C. Generate a short action strip

```text
Edit the supplied layout using the approved seed in reference A.
Create [four] chronological frames of [one action], facing [direction]
throughout, in [four] equal horizontal slots.
Pose plan: [anticipation], [contact/impact], [follow-through], [recovery].
Keep the same camera, character scale, anatomy, costume and material.
Preserve [named identity landmarks]. Keep root [fixed/in specified offsets].
Do not center each silhouette independently. Leave room for the entire action.
[Contact foot/paw] stays planted during [specified frames].
No camera motion, background, labels, speed blur or baked effect trail.
Transparent output. This is one action, not four character variations.
```

For a walk, replace the pose plan with the intended gait and contacts. Generate a cycle whose timing is authored after review; do not assume equally spaced output poses imply correct timing.

### D. Extend an environment kit

```text
Reference A is the approved room and camera. Reference B defines the
material treatment. Produce proposals for [one prop family], matching
[scale cue], light direction, outline and palette roles.
These belong to [specific place/culture/function] and should share
[recurring construction motif]. Variations must differ in [useful feature].
Show each full object separated with generous margins on transparency.
No scenery or presentation shadows. Provide [front/quarter] view only.
For the [door/chest] preserve identical construction in [closed/open] states.
```

For tiles, add exact edge/corner requirements and a supplied template, then manually verify seams and transition coverage. Do not label a generated grid engine-ready before export and assembly tests.

## A small pilot that decides the pipeline

Use one asymmetric hero, one interactable prop, and one room kit. The asymmetry deliberately tests whether opposing views are truly controlled. Choose the actual intended resolution and camera first.

1. Produce the same directional idle and short walk using **reference-guided AI plus cleanup** and **one controllable alternative**: manual key frames, a layered rig, or 3D-to-2D depending on the game.
2. Include front, rear and quarter views if those are required by the camera. Include one interaction with a clear contact point and one turn or action transition.
3. Assemble one room with foreground occlusion and test the character at game size. Reuse the kit for a second layout rather than generating another complete room illustration.
4. Ask for a design change after the first version: change a costume detail and retime an action. Measure the revision cost across directions.

Log the following for each route, including failed attempts:

| Measure                                               | Why it matters                                          |
| ----------------------------------------------------- | ------------------------------------------------------- |
| Setup, generation, review, cleanup and import minutes | Total accepted-asset cost, rather than generation speed |
| Attempts and tool cost per accepted action/view       | Failure rate and predictability                         |
| Frames with identity or anatomical errors             | Whether approval scales to the planned inventory        |
| Unintended root jitter, foot sliding, scale change    | Whether motion survives real playback                   |
| Direction coverage and mirror exceptions              | Hidden workload for asymmetric characters               |
| Revision minutes across the whole character           | Whether the pipeline supports game iteration            |
| Atlas/metadata failures and memory footprint          | Whether the images fit runtime requirements             |
| Blind visual preference at game scale                 | Whether process efficiency preserves the intended art   |

Define thresholds before evaluating. Example: no missing states, no accidental costume-side swaps, no unintended scale changes, every contact readable, and a declared cleanup budget per accepted action. Numerical pixel tolerances must follow the actual resolution and intended motion; there is no universal correct value. Do not punish intentional squash/stretch or airborne motion as jitter.

Choose the route with the best **accepted quality, revision cost and throughput** for the game. A single beautiful still, a polished vendor demo, or a normalization script cannot answer that decision. If manual repair repeatedly dominates production, simplify the design/directions, change the animation method, or reduce the asset scope before scaling.

## Confidence and remaining uncertainties

**High confidence:** concept references and runtime atlases are different deliverables; generative consistency needs verification; fixed technical contracts and actual motion previews are essential; established 2D, rigged and 3D-to-2D routes provide controllable alternatives.

**Medium confidence:** short, seed-guided strip generation will be useful for selected actions and styles. It is a well-specified tactic in the inspected skill, but its benefit for a particular character must be measured.

**Unresolved until a pilot:** which current model/service best preserves the selected material, how many views are affordable, whether training is worthwhile, how much paintover is needed, and whether 3D setup pays back at the planned asset count. No controlled comparative benchmark or generated production trial was performed for this note.
