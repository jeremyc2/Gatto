# Building TypeScript games with current Phaser

Research date: October 18, 2026 (America/Chicago). Scope: **Phaser 4.2.1 “Giedi,” the latest stable published Phaser release**, released July 9, 2026. This is a current-version research guide, not an implementation or a decision to add Phaser to Gatto. Other research and application files on this branch are outside this note's scope.

The official download page and GitHub release identify 4.2.1; the package registry independently reports `dist-tags.latest: 4.2.1`, published `2026-07-09T14:35:35.104Z`, with source revision `41be1e462bc600064e498cba370bfa8c5c055a22`. All source links below are pinned to that release. “Latest” here means this checked stable package, not repository HEAD, an experimental branch, or Phaser Editor's separately numbered releases. [Official download](https://phaser.io/download/release/v4.2.1), [release](https://github.com/phaserjs/phaser/releases/tag/v4.2.1), [registry metadata](https://registry.npmjs.org/phaser).

**Recommendation:** use Phaser's scene lifecycle, asset manager, cameras, input, animation, audio, and an appropriate physics system as the foundation. Use its current WebGL renderer and filters for presentation. Reach for GPU layers, Mesh2D, stencils, or custom render nodes when a specific visual or measured workload calls for them. Most games benefit more from clear ownership and a small playable loop than from activating every renderer feature. These priorities and the architecture proposals below are this guide's synthesis, rather than a framework-imposed structure.

## 1. Establish the version boundary first

The package includes its own TypeScript declarations at `types/phaser.d.ts`. Its public package entry exports the ESM build to `import`, the bundled build to `require`, and declarations to TypeScript. Do not add a separately maintained Phaser typings package. Phaser itself is distributed as JavaScript with declarations; writing a game in TypeScript does not require the engine to be authored in TypeScript. [4.2.1 package manifest](https://github.com/phaserjs/phaser/blob/v4.2.1/package.json), [bundled declarations](https://github.com/phaserjs/phaser/blob/v4.2.1/types/phaser.d.ts).

The public API documentation landing page identified itself as **4.1.0** when checked, and search results also expose other versioned documentation. Useful conceptual explanations can therefore lag the current package. For a 4.2.1 API or signature, check the installed declaration and pinned source before adopting an example. In particular, the current renderer uses **render nodes, filters, `setLighting`, and the new game objects** documented here. A tutorial is not current merely because a search engine crawled it recently. [API landing page](https://docs.phaser.io/api-documentation/api-documentation), [release rendering guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Rendering%20Concepts/Phaser%204%20Rendering%20Concepts.md).

The official Vite TypeScript template is useful as a file-layout reference, but its manifest still pinned Phaser **4.0.0** when checked. Its scripts also include a separate logging step. Do not copy that manifest unchanged into a latest-only, Bun-based project; explicitly pin 4.2.1 and keep only the tooling you intend to run. [Official template manifest](https://github.com/phaserjs/template-vite-ts/blob/main/package.json).

## 2. A small Bun, Vite, and strict TypeScript foundation

For a **new, separate game project**, the following is a proposed setup, not a command executed in this repository. Bun supplies the runtime, package manager, and script execution; Vite supplies the website development server and production build. Pin Phaser exactly and commit `bun.lock` so another agent or developer gets the same engine. Check current compatible tooling versions when actually creating the project; this guide pins the engine rather than making unverified claims about the latest Vite or TypeScript release.

```sh
bun init -y
bun add --exact phaser@4.2.1
bun add -d vite typescript
```

Use these scripts in `package.json`:

```json
{
  "scripts": {
    "dev": "bunx --bun vite",
    "check": "bunx --bun tsc --noEmit",
    "build": "bun run check && bunx --bun vite build",
    "preview": "bunx --bun vite preview"
  }
}
```

A minimal `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "types": ["vite/client"],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "isolatedModules": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["src"]
}
```

Vite transpilation is not a substitute for type checking, so the build script runs the checker explicitly. `skipLibCheck` skips checking dependency declarations against one another; it does not make application code untyped. The namespace import in the example below matches Phaser's declaration/export shape without a synthetic-default-import assumption. [Vite TypeScript behavior](https://vite.dev/guide/features.html#typescript), [TypeScript compiler options](https://www.typescriptlang.org/tsconfig/), [Phaser package entry](https://github.com/phaserjs/phaser/blob/v4.2.1/package.json).

Create `index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Phaser game</title>
  </head>
  <body>
    <div id="game"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

This complete `src/main.ts` example has no external artwork. It generates a texture, creates a lit Arcade Physics object, uses normalized arrow-key movement, and demonstrates a current cone light:

```ts
import * as Phaser from "phaser";

class Play extends Phaser.Scene {
  private hero?: Phaser.Physics.Arcade.Image;
  private cursors?: Phaser.Types.Input.Keyboard.CursorKeys;
  private readonly direction = new Phaser.Math.Vector2();

  constructor() {
    super("play");
  }

  create(): void {
    if (!this.textures.exists("hero")) {
      const brush = this.add.graphics();
      brush.fillStyle(0xffce73);
      brush.fillRect(0, 0, 24, 24);
      brush.generateTexture("hero", 24, 24);
      brush.destroy();
    }

    this.hero = this.physics.add
      .image(320, 180, "hero")
      .setCollideWorldBounds(true)
      .setLighting(true);
    this.cursors = this.input.keyboard?.createCursorKeys();

    this.lights.enable().setAmbientColor(0x303030);
    this.lights.addConeLight(320, 180, 260, 0xffe3b0, 2, 0, Math.PI / 6, Math.PI / 3);

    this.events.once(Phaser.Scenes.Events.SHUTDOWN, () => {
      this.hero = undefined;
      this.cursors = undefined;
    });
  }

  update(): void {
    const hero = this.hero;
    const cursors = this.cursors;
    if (!hero || !cursors) return;

    this.direction.set(
      Number(cursors.right.isDown) - Number(cursors.left.isDown),
      Number(cursors.down.isDown) - Number(cursors.up.isDown),
    );
    this.direction.normalize().scale(180);
    hero.setVelocity(this.direction.x, this.direction.y);
  }
}

const config: Phaser.Types.Core.GameConfig = {
  type: Phaser.WEBGL,
  parent: "game",
  width: 640,
  height: 360,
  backgroundColor: "#151a25",
  scale: {
    mode: Phaser.Scale.FIT,
    autoCenter: Phaser.Scale.CENTER_BOTH,
  },
  physics: {
    default: "arcade",
    arcade: { gravity: { x: 0, y: 0 }, debug: false },
  },
  scene: [Play],
};

new Phaser.Game(config);
```

The movement speed is in physics units per second. Do not multiply `setVelocity` by frame delta: the physics simulation integrates velocity. The reused vector avoids a new allocation each frame and normalization prevents diagonal movement from being faster. The stationary cone is intentional; following a player means updating the light's position and cone rotation. Optional keyboard access handles the declared possibility that keyboard support is disabled; a shipped game would add touch/gamepad input or an explicit required-device policy. [Arcade Body](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/Body.js), [Vector2](https://github.com/phaserjs/phaser/blob/v4.2.1/src/math/Vector2.js), [cone-light guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Cone%20Lights/Phaser%204%20Cone%20Lights.md).

## 3. Let scenes own game lifetime

A scene has its own systems, display/update lists, cameras, input, loader, timers, and optional physics integration. `preload` queues assets, the loader completes, `create` constructs the playable scene, and `update(time, delta)` runs during active scene steps. The scene's system step emits pre-update, update, and post-update events around the scene callback. These are lifecycle boundaries, not just convenient method names. [Scene systems](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/Systems.js), [scene startup](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/SceneManager.js).

Recommended structure for a growing game:

```text
src/
  main.ts                   # One Game instance and configuration
  scenes/                   # Boot, menu, play, HUD, results
  features/player/          # Controls, state, spawning, presentation
  features/combat/          # Damage, projectiles, collision policy
  features/world/           # Map loading, level content, transitions
  shared/                   # Small stable data types and utilities
  platform/                 # Save storage, host UI, telemetry adapters
```

Use scenes for meaningful lifetimes or independently pausable presentation layers. A HUD scene can remain active over a paused gameplay scene. Do not create a scene per enemy or make a single scene own all progression, save data, UI, and combat rules. Keep reusable gameplay rules in ordinary typed functions or small modules; let the scene adapt those rules to Phaser objects. This is an architectural recommendation supported by the engine's independent scene execution and pause/sleep/stop controls. [ScenePlugin controls](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/ScenePlugin.js).

Reset run-specific fields on each start. A stopped scene can start again without constructing a new scene instance. Use `SHUTDOWN` for subscriptions and resources belonging to one run; reserve `DESTROY` for final scene removal. Remove handlers attached to long-lived game emitters, DOM targets, sockets, or external stores, and cancel owned asynchronous work. Phaser can clean up scene-managed objects; it cannot infer the ownership of your unrelated callbacks. [Scene shutdown/destruction](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/Systems.js), [scene restart behavior](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/SceneManager.js).

Textures, asset caches, and the Animation Manager are game-level resources shared across scenes. Give assets and animation definitions stable keys; create global animations once or check for an existing key. Removing a shared texture to clean up one level can invalidate objects in another level. Define explicit session-level and level-level ownership. [Loader resource scope](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/LoaderPlugin.js), [Animation Manager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/animations/AnimationManager.js).

## 4. Choose physics for the actual mechanic

Both built-in systems below are present in 4.2.1. Their continued availability does not make their APIs obsolete; choose between current systems on behavior and workload.

| Requirement                                                            | Recommended starting point                               | Reason and constraint                                                                                   |
| ---------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Fast platformer, top-down character, bullets, straightforward overlaps | Arcade Physics                                           | Rectangular/circular collision with simple separation; good fit for authored arcade controls            |
| Rotating bodies, constraints, irregular/compound shapes, stacking      | Matter Physics                                           | Full rigid-body model, constraints, material properties, collision queries, and stable resting/stacking |
| A highly specific controller or unusual movement law                   | Explicit simulation with a deliberate collision boundary | A design choice when owning movement rules is useful; avoid two systems both writing the same transform |

Arcade's own source cautions that its separation method is unsuitable for demanding stacks and dense resting contacts. Matter offers those richer interactions at a different complexity and tuning cost. These are fit recommendations, not a claim that one engine is universally faster or guarantees a particular frame rate. [Arcade plugin](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/ArcadePhysics.js), [Matter plugin](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/matter-js/MatterPhysics.js).

Practical current-system choices:

- Register persistent colliders/overlaps during setup, rather than creating a new collider every frame. Keep a clear distinction between collision response and trigger detection. [Arcade factory](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/Factory.js).
- Use static bodies for immovable collision geometry; after repositioning a static object's visual transform, deliberately synchronize its body. Pooling a body also requires resetting velocity, enabled state, collision flags, and game-specific hit state. [StaticBody](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/StaticBody.js), [dynamic Body reset](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/Body.js).
- Arcade defaults to `fixedStep: true` and `fps: 60`. That is its physics cadence, not proof that all game logic is fixed-step or deterministic. Its update subscription and the scene callback have an order; define which phase samples input and applies simulation changes when exact tick behavior matters. [World defaults and stepping](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/World.js), [plugin event hooks](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/ArcadePhysics.js).
- For custom motion, `delta` is milliseconds; convert it to seconds before multiplying a pixels-per-second speed. For replay/rollback, introduce an explicit input/tick/state contract and verify it. Phaser's smoothed browser loop by itself is not that contract. [TimeStep](https://github.com/phaserjs/phaser/blob/v4.2.1/src/core/TimeStep.js).

## 5. Use the current WebGL renderer as the default

Phaser 4.2.1's WebGL renderer is built around **RenderNodes** and managed WebGL state. Most gameplay code does not need to handle nodes. Ordinary images, sprites, text, lights, and filters already use the system. The Canvas renderer remains present but is deprecated and does not provide the advanced WebGL feature set. Choose `Phaser.WEBGL` when those features are part of the game's requirements; handle a renderer-initialization failure in the host experience. [Release rendering concepts](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Rendering%20Concepts/Phaser%204%20Rendering%20Concepts.md), [renderer creation](https://github.com/phaserjs/phaser/blob/v4.2.1/src/core/CreateRenderer.js).

The renderer normally requests WebGL and can also handle a supplied WebGL2 context; this does not imply automatic WebGL2 or WebGPU selection. Managed context restoration is a reason to test background/resume and context loss on real devices, not a guarantee that arbitrary custom GPU resources or every plugin will restore correctly. Use Phaser's wrappers and retained resource abstractions for custom rendering rather than issuing unmanaged raw `gl` calls that invalidate tracked state. [WebGLRenderer context creation and compatibility handling](https://github.com/phaserjs/phaser/blob/v4.2.1/src/renderer/webgl/WebGLRenderer.js), [current shader guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Shader%20Guide/Phaser%204%20Shader%20Guide.md).

For performance, keep compatible draws adjacent where visual ordering permits, reuse atlases/textures, and avoid unnecessary changes of blend mode, lighting path, or filter target. Do not reorder a transparent foreground behind an enemy just to reduce draw calls. Judge rendering cost with a representative scene and target device: sprite count alone omits texture uploads, fragment work, overdraw, physics, and scripting. These are optimization recommendations derived from the renderer's batching and filter mechanisms. [Batch handler](https://github.com/phaserjs/phaser/blob/v4.2.1/src/renderer/webgl/renderNodes/BatchHandler.js), [Filters cost model](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/Filters.js).

## 6. Filters: one system for effects, compositing, and masks

Game-object filters require `enableFilters()`. The object then exposes `filters.internal` and `filters.external`; camera filters are also available. Internal processing follows object space/bounds where possible, whereas external processing uses its drawing context. Filters render through textures/framebuffers and add passes. Applying one effect to a cohesive container or camera can be a useful alternative to applying it separately to hundreds of children, provided the resulting appearance is correct. [Filters component](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/Filters.js), [FilterList](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/FilterList.js).

This is an independent continuation inside `create`, assuming the complete example's `hero` texture exists:

```ts
const badge = this.add.image(100, 80, "hero");
badge.enableFilters();
badge.filters?.internal.addGlow(0x78caff, 2, 0);

// A separate tint example; does not need a filter framebuffer.
const inverted = this.add.image(150, 80, "hero");
inverted.setTint(0x000000).setTint2(0xffffff).setTintMode(Phaser.TintModes.MULTIPLY_TWO);
```

Useful current tools include `Mask` for alpha masking, `GradientMap` for palette treatment, `Quantize` for reduced-color looks, `ImageLight` for normal/environment-based lighting, and `Blend` for texture compositing beyond native WebGL blend modes. Bloom and shine are available through `Phaser.Actions.AddEffectBloom` and `AddEffectShine`, which assemble their component operations. Consult each current function signature before copying effect code. [Current filter exports](https://github.com/phaserjs/phaser/blob/v4.2.1/src/filters/index.js), [bloom action](https://github.com/phaserjs/phaser/blob/v4.2.1/src/actions/AddEffectBloom.js), [shine action](https://github.com/phaserjs/phaser/blob/v4.2.1/src/actions/AddEffectShine.js), [two-color tint implementation](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/Tint.js).

Recommendations: budget filter passes, padding, framebuffer dimensions, and nesting before adding several screen-wide effects. Static treatment can sometimes be baked into an asset or retained texture. A glow clipped at its edges may be a bounds/padding issue; a blurred fullscreen composition is not equivalent in cost or appearance to one small blurred sprite. The exact result depends on the filter's implementation and selected context. [FilterList padding and processing](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/FilterList.js), [filter focus/bounds](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/Filters.js).

Other useful composition tools:

- `CaptureFrame` captures content already drawn at its position in render order, making scoped water/distortion compositions possible. It needs a framebuffer context, for example a camera with `setForceComposite(true)`; it does not draw an image itself. [CaptureFrame](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/captureframe/CaptureFrame.js).
- `DynamicTexture` retains a composite for reuse by sprites or other consumers. Redraw its content after WebGL context restoration; the application owns that reconstruction. [DynamicTexture](https://github.com/phaserjs/phaser/blob/v4.2.1/src/textures/DynamicTexture.js).
- `Gradient` and `Noise` are shader-based visual objects suitable for authored color ramps, atmosphere, or mask/filter sources. Consider them when a procedural visual layer is useful, rather than automatically loading another bitmap. [Gradient](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/gradient/Gradient.js), [Noise](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/noise/Noise.js).

## 7. Lighting, including directional cone lights

Enable the scene Light Manager with `this.lights.enable()` and opt receiving objects into lighting with **`setLighting(true)`**. Normal maps are optional; the default normal is flat. Use ambient color for base illumination and a bounded number of local lights for detail. `addConeLight` uses the same lighting path and can create a flashlight without a second camera or a mask-based re-render. Rotations are radians and cone angles are **full cone widths**, with an inner fully lit area and an outer falloff. A Point Light game object is a separate choice for a radial glow that does not illuminate neighbors. [4.2.1 cone-light guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Cone%20Lights/Phaser%204%20Cone%20Lights.md), [LightsManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/lights/LightsManager.js), [Lighting component](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/Lighting.js).

Recommendation: prototype one player light and a few local lights first. Measure large overlapping radii and the configured visible-light limit. A cone is a lighting effect; it does **not** implement enemy perception, line-of-sight collision, or automatic shadow casting. Those need separate gameplay geometry or an explicitly authored rendering technique. The distinction follows the light's shader/configuration API, which describes illumination rather than a visibility query. [Light](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/lights/Light.js), [light culling](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/lights/LightsManager.js).

## 8. Mesh2D: deformable 2D artwork and custom geometry

`Mesh2D` contains textured triangles. It supports batching with compatible images, sprites, and text, and supports the lighting path. It is a **2D** facility, without 3D depth. Vertices encode `x, y, u, v`; each index group encodes three vertex indices plus a texture-page index. UVs are your responsibility rather than automatically using sprite frame geometry. [Mesh2D](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/mesh2d/Mesh2D.js), [factory](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/mesh2d/Mesh2DFactory.js).

Choose its render strategy deliberately:

- **Stable topology, moving vertices:** build the ordered index list once with `buildOrderedIndices`, which pairs triangles into quads for the regular batch path. This suits a waving banner, soft prop, or custom deformation.
- **Frequently changing connectivity:** `renderAsTriangles()` avoids rebuilding quad ordering but uses a separate triangle render node, losing batching with normal quads.
- **Default unordered indices:** each triangle occupies a quad; the source explicitly identifies this as less efficient. Do not assume construction alone selected an optimized topology.

These distinctions are source-defined behavior, not a benchmark. A mesh visual also does not automatically create a matching physics body. Keep deformation and collision models deliberately related. [Mesh2D strategies](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/mesh2d/Mesh2D.js).

If using Spine, the official integration announcement identifies `@esotericsoftware/spine-phaser-v4` **4.3.11** as the first version of its new backend requiring Phaser **4.2.1+**. That backend uses Mesh2D and the stencil APIs for compatible batching and clipping attached Phaser objects. This establishes a minimum compatibility boundary; it is not a claim that 4.3.11 is today's latest Spine runtime. Match the runtime's major/minor version to the Spine Editor version used to export skeletons, and verify the current plugin before adopting it. Spine also has its own runtime license, separate from Phaser's MIT license. [Official integration announcement](https://phaser.io/news/2026/07/phaser-4-2-spine-renderer-mesh2d-stencil), [Spine runtime compatibility and export requirements](https://esotericsoftware.com/spine-phaser), [Spine runtime license](https://esotericsoftware.com/spine-runtimes-license), [Phaser license](https://github.com/phaserjs/phaser/blob/v4.2.1/LICENSE.md).

## 9. Stencils and AlphaStrategy: efficient hard cutouts

`Stencil` is a special container whose children write into the stencil buffer to affect later draws. Its result persists for the remaining frame. `StencilReference` can render the same source geometry again with different rules to remove or reuse that region. This makes a stencil useful when one hard-edged region should affect many subsequent objects. A `Mask` filter is the better starting point for smooth alpha and masks embedded in a filter chain. [Stencil](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/stencil/Stencil.js), [StencilReference](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/stencilreference/StencilReference.js).

Important current details:

- Stencil is binary. Transparent source fragments must actually be discarded; the stencil buffer does not honor alpha blending. Default stencil alpha handling uses dithering.
- `AlphaStrategy` is exactly **`'keep' | 'dither' | number`**. A number between 0 and 1 supplies a threshold; the string `'threshold'` is not the declared API.
- A shader that does not support alpha strategies can still write its transparent area into a stencil. `stencilCompositeCheck: true` forces a compatible composite path at an extra rendering cost.
- Overlapping geometry can increment a pixel more than once. The stencil has eight bits; uncontrolled layering or subtraction can wrap values. Keep the region/ordering protocol explicit and avoid unnecessary nested stencils.
- `render.stencil: false` saves stencil attachments but disables these objects. The game also supports `render.alphaStrategy` and `render.stencilAlphaStrategy`; changing these globally changes semantics across many draws.

[Exact AlphaStrategy typedef](https://github.com/phaserjs/phaser/blob/v4.2.1/src/renderer/webgl/typedefs/AlphaStrategy.js), [stencil options and failure modes](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/stencil/Stencil.js), [render configuration](https://github.com/phaserjs/phaser/blob/v4.2.1/src/core/typedefs/RenderConfig.js).

`CustomContext` is an advanced container for adjusting a cloned `DrawingContext` before rendering its children. Prefer its supported setters. Retain a `DynamicTexture` when a persistent offscreen target is needed; creating a framebuffer every frame is explicitly discouraged by the source. This is a targeted rendering escape hatch, not a default gameplay abstraction. [CustomContext](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/customcontext/CustomContext.js).

## 10. GPU layers: use the constraints to get the benefit

| Current object                 | Best starting use                                                                         | Critical constraints                                                                                                                 |
| ------------------------------ | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `SpriteGPULayer`               | Dense decorative backgrounds, foliage, stars, or repeated props with GPU-driven animation | One single-image texture, no multi-atlas; editing membership/data triggers buffer work; members are not ordinary interactive sprites |
| `TilemapGPULayer`              | Large mostly unchanged orthographic layers, including zoomed-out maps                     | One tileset/image; up to 4096 × 4096 tiles; orthographic only; edits require explicit GPU data regeneration                          |
| Ordinary Sprite / TilemapLayer | Interactive actors, frequently changing content, flexible maps                            | More CPU-side work, but easier object-level behavior and flexible map rendering                                                      |

`SpriteGPULayer` stores member render data in GPU buffers and can animate properties without rewriting that data every frame. Populate coherently and keep data stable; frequent add/remove/patch operations undermine that advantage. Drawn area still costs fragments, so a huge transparent or overlapping quad population can become fill-rate limited. Published “million sprite” or speed-multiplier language is not an independently reproduced guarantee for your game. [4.2.1 SpriteGPULayer](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/spritegpulayer/SpriteGPULayer.js).

`Tilemap.createLayer(layerID, tileset, x, y, true)` requests the GPU layer. The layer draws using tile-data textures and a single screen-covering grid representation, rather than per-tile CPU draw work. It supports tile flips and animation, but editing tile data does not update its GPU texture automatically: call `generateLayerDataTexture()`. Retain ordinary layers where the map needs their flexibility; a decorative GPU layer is not a substitute for explicitly configuring gameplay collision. [Tilemap factory method](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tilemaps/Tilemap.js), [TilemapGPULayer](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tilemaps/TilemapGPULayer.js).

The release's rendering guide recommends this layer for large visible tile counts, but also flags its one-quad shader path as inefficient on some mobile devices. Treat it as a benchmark candidate on your target phones, rather than assuming that moving work to the GPU always wins. [TilemapGPULayer runtime-cost caveat](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Rendering%20Concepts/Phaser%204%20Rendering%20Concepts.md#tilemapgpulayer-1).

## 11. Asset, animation, input, audio, and camera choices

### Assets and maps

Use `preload` and the built-in loader for images, atlases, Aseprite exports, Tiled JSON, audio, and data. Loading outside `preload` requires starting the loader explicitly. Show progress/error UI, and split a small startup payload from level-specific loads. Because resources are cached at game scope, build a key/ownership convention before adding streamed levels. These loading priorities are recommendations; the loader provides the queue, parallel loading, and events. [LoaderPlugin](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/LoaderPlugin.js), [file-type inventory](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/filetypes/index.js).

Use an atlas to organize related frames and animation rather than scattering many unrelated texture loads. Extrude/pad frames when linear filtering could sample neighboring artwork. For compressed textures, follow the current package's format/fallback guidance and GL orientation requirements; validate actual devices before choosing a single compressed format. A small downloaded file is not evidence of a small decoded GPU footprint. [Atlas loader](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/filetypes/AtlasJSONFile.js), [compressed-texture guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/WebGL%20Compressed%20Textures/WebGL%20Compressed%20Textures%20in%20Phaser.md).

Use Tiled object layers as authored spawn/content data with validation, and explicitly configure collision indexes/properties on collision layers. Rendering a wall graphic does not itself establish a collision rule. Keep map parsing separate from gameplay spawning so invalid content is diagnosed during load rather than deep inside combat. [Tilemap](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tilemaps/Tilemap.js), [TilemapLayerBase](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tilemaps/TilemapLayerBase.js).

### Animation, tweens, and particles

Use sprites and the Animation Manager for frame animation, including Aseprite-tag import. Use tweens for authored presentation transitions and particles for bounded transient effects. Define gameplay state independently of whether a visual effect has finished; when a mechanic intentionally depends on an animation event, test interruption, scene shutdown, and replay paths. Global animation keys deserve the same ownership care as shared textures. [AnimationManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/animations/AnimationManager.js), [TweenManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tweens/TweenManager.js), [ParticleEmitter](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/particles/ParticleEmitter.js).

Use a Group when a reusable pool is valuable for projectiles or effects; Group membership is not a transform hierarchy. A Container is the object for shared transforms. Reset all reused gameplay state, not just visibility, and bound particle lifespan/count so an effect cannot silently accumulate forever. [Group](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/group/Group.js), [Container](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/container/Container.js), [particle limits](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/particles/ParticleEmitter.js).

### Input and cameras

Translate keyboard, pointer, touch, and gamepad signals into a small action model such as `move`, `jumpPressed`, `aim`, and `pausePressed`. This makes remapping and alternate controls easier to test. Poll held movement; use edge-triggered semantics such as `JustDown` for one-shot actions. Decide which concurrently running scene receives input; a visible menu must not leave gameplay responding underneath. [InputPlugin](https://github.com/phaserjs/phaser/blob/v4.2.1/src/input/InputPlugin.js), [KeyboardPlugin](https://github.com/phaserjs/phaser/blob/v4.2.1/src/input/keyboard/KeyboardPlugin.js), [JustDown](https://github.com/phaserjs/phaser/blob/v4.2.1/src/input/keyboard/keys/JustDown.js).

Cameras provide follow, bounds, zoom, viewport, effects, and object exclusion. Use `camera.getWorldPoint` when pointer coordinates must become world positions under camera transforms. A minimap or extra camera renders another view, so measure it. Keep HUD and gameplay camera ownership explicit, especially when a pause overlay or multiple viewports are present. [Camera](https://github.com/phaserjs/phaser/blob/v4.2.1/src/cameras/2d/Camera.js), [BaseCamera coordinate conversion](https://github.com/phaserjs/phaser/blob/v4.2.1/src/cameras/2d/BaseCamera.js).

### Audio and pausing

The Web Audio manager handles browser audio unlocking through user interaction. Build a clear start interaction and verify the first sound on mobile, rather than assuming that a `play` request during initial loading is audible. The shared sound manager defaults to pausing on blur, but pausing a gameplay scene is not a complete policy for global music, host UI, saving, or all external work. Define what your pause/menu actually owns. [WebAudioSoundManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/sound/webaudio/WebAudioSoundManager.js), [BaseSoundManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/sound/BaseSoundManager.js).

## 12. Pixel art and viewport policy

For artwork that should remain crisp while enlarged and rotated, consider **`render.smoothPixelArt: true`**. For native-size aligned pixel art, consider **`render.pixelArt: true`** and avoid arbitrary scale, rotation, and zoom. They serve different goals; enabling every “pixel” setting is not a coherent policy. Camera transforms, fractional positions, sprite origins, and texture filtering all affect the result. The current guide itself contains an inconsistent Pixelate/Blocky example, so check the actual FilterList method before adopting that particular line. [4.2.1 pixel-art guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Pixel%20Art%20Guide/Phaser%204%20Pixel%20Art%20Guide.md), [FilterList signatures](https://github.com/phaserjs/phaser/blob/v4.2.1/src/gameobjects/components/FilterList.js).

Choose a logical gameplay viewport and a resize policy intentionally. `FIT` scales a fixed game size to fit available space; a responsive `RESIZE` game requires layout/camera/UI adaptation. Test wide, tall, low-height, and high-density screens with actual controls. More render pixels increase fragment and filter work; unconditionally adopting device pixel ratio can spend a large GPU budget without improving the game's intended style. These are recommendations derived from the scale modes and per-pixel render costs. [Scale modes](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scale/const/SCALE_MODE_CONST.js), [ScaleManager](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scale/ScaleManager.js), [TilemapGPULayer pixel-cost model](https://github.com/phaserjs/phaser/blob/v4.2.1/src/tilemaps/TilemapGPULayer.js).

## 13. Production verification and debugging

The following is a proposed practical verification workflow, not a claim that Phaser ships a turnkey testing service:

1. **Typecheck and build.** Run `bun run check` and `bun run build`; preview the built output so asset paths and bundler behavior match deployment. Check subpath hosting if the game will not live at `/`. Vite's public directory copies files without module-graph hashing, whereas imported assets participate in its build pipeline. Choose one asset strategy deliberately. [Vite asset handling](https://vite.dev/guide/assets.html), [Phaser URL loading](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/LoaderPlugin.js).
2. **Test rules outside the renderer.** Use Bun tests for inventory, damage, progression, deterministic movement helpers, save migrations, and action mapping where they are ordinary data/functions. Avoid pretending a server-side unit test proves WebGL, audio, or touch behavior.
3. **Exercise real browser flows.** Load → play → pause → resume → restart → change level → return to menu. Repeat to expose retained global listeners, animation-key duplication, sounds, textures, timers, and external callbacks. Test loading failures and unavailable assets explicitly. [Scene lifecycle](https://github.com/phaserjs/phaser/blob/v4.2.1/src/scene/Systems.js), [shared cache behavior](https://github.com/phaserjs/phaser/blob/v4.2.1/src/loader/LoaderPlugin.js).
4. **Inspect collisions visually.** Enable the chosen physics system's debug rendering during diagnosis, reproduce edge cases, and disable it for release measurements. Verify fast projectiles, moving platforms, body/visual alignment, and pooled objects for the actual game. [Arcade debug settings](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/arcade/typedefs/ArcadeWorldConfig.js), [Matter debug settings](https://github.com/phaserjs/phaser/blob/v4.2.1/src/physics/matter-js/typedefs/MatterDebugConfig.js).
5. **Profile a representative worst case.** Measure scripting/physics, rendering, draw calls/uploads, texture memory, allocation/GC, loading/decoding, and frame-time spikes separately. Compare before/after with the same content and device. Introduce GPU layers, pooling, atlas changes, or reduced filter scope against the bottleneck you found. [Browser performance recording](https://developer.chrome.com/docs/devtools/performance/overview), [current renderer](https://github.com/phaserjs/phaser/blob/v4.2.1/src/renderer/webgl/WebGLRenderer.js).
6. **Test device lifecycle.** Verify audio after an interaction, background/resume, visibility changes, context restoration, resize/orientation, and a complete game restart. Include a representative low-end target rather than treating desktop results as a mobile guarantee. [renderer restoration](https://github.com/phaserjs/phaser/blob/v4.2.1/src/renderer/webgl/WebGLRenderer.js), [audio unlocking](https://github.com/phaserjs/phaser/blob/v4.2.1/src/sound/webaudio/WebAudioSoundManager.js).

The current `game.loop.setFPSLimit(limit)` permits changing a configured frame-rate limit at runtime. It may be useful for a quiet menu or power-conscious mode; verify gameplay and physics behavior when changing it. A frame-rate target is not proof that a device sustains it, and physics cadence is a separate concern. [TimeStep#setFPSLimit](https://github.com/phaserjs/phaser/blob/v4.2.1/src/core/TimeStep.js).

## 14. Adoption sequence and current-only reference shelf

Recommended sequence:

1. Build one mechanic using scenes, input, current physics, a camera, and placeholder visuals.
2. Establish lifecycle, asset keys, action mapping, save boundaries, and restart behavior.
3. Develop the visual style with current filters, tint, lighting, and the chosen pixel/viewport policy.
4. Add Mesh2D, stencils, procedural gradients/noise, or GPU layers where they make a visible design difference.
5. Profile the busiest playable scene and optimize the expensive part; only then consider custom render nodes.

For future reading, start with these current release artifacts:

| Source                                                                                                                                                           | What it is best for                                                                                  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| [4.2.1 release](https://github.com/phaserjs/phaser/releases/tag/v4.2.1)                                                                                          | Exact installed release and patch fixes, including ESM, resize, stencil, and tween-delay corrections |
| [4.2.1 declarations](https://github.com/phaserjs/phaser/blob/v4.2.1/types/phaser.d.ts)                                                                           | Actual TypeScript interface of the pinned package                                                    |
| [4.2.1 rendering concepts](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Rendering%20Concepts/Phaser%204%20Rendering%20Concepts.md)           | Render architecture, current objects, filter concepts, GPU-layer model                               |
| [4.2.1 shader guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Shader%20Guide/Phaser%204%20Shader%20Guide.md)                             | Managed WebGL state, shader/filter authoring, render nodes                                           |
| [4.2.1 internal-space guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Internal%20Space%20Guide/Phaser%204%20Internal%20Space%20Guide.md) | Coordinates, internal/external filters, masking and compositing                                      |
| [4.2.1 pixel-art guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Pixel%20Art%20Guide/Phaser%204%20Pixel%20Art%20Guide.md)                | Choosing smooth or rounded pixel rendering; inspect API signatures for examples                      |
| [4.2.1 cone-light guide](https://github.com/phaserjs/phaser/blob/v4.2.1/docs/Phaser%204%20Cone%20Lights/Phaser%204%20Cone%20Lights.md)                           | Complete cone-light semantics and examples                                                           |
| [4.2.1 source tree](https://github.com/phaserjs/phaser/tree/v4.2.1/src)                                                                                          | Source authority when a live documentation page or example disagrees                                 |

### Research verification and limits

The published `phaser@4.2.1` archive was downloaded to a temporary directory and inspected for its manifest, declarations, release guides, and relevant subsystem source. Registry version/provenance and the official release were checked independently. The complete TypeScript scene/configuration example passed strict checking against the published package under both TypeScript 6.0.2 and the repository's TypeScript 7.0.2, using Bun. The main example and filter/tint continuation also passed with `strict`, `noUncheckedIndexedAccess`, and `skipLibCheck` enabled under 7.0.2; this checks application usage, not the internals of dependency declarations. No project dependency was installed or changed. The code above is original teaching code, not copied game content. Browser rendering, device performance, gameplay quality, and third-party plugin compatibility require a separate playable implementation and were not measured in this research.
