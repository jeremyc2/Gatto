# Using current bitECS in a growing TypeScript game

Research date: October 18, 2026 (America/Chicago). Scope: the current published `bitecs` package, **0.4.0**, its supported modern API, data model, storage, queries, relationships, observers, prefabs, serialization, concurrency, game organization, performance, and maintainability. This is a research guide, not an implementation or a proposal to add a dependency to Gatto.

The live package registry reports `latest: 0.4.0`, published December 6, 2025. The website identifies its documentation as 0.4. The repository's published bundles match the inspected source revision [`1585cdb4ae9a6586aef5e4d9d30d7039e22cd543`](https://github.com/NateTheGreatt/bitECS/tree/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543). These are separate checks: repository HEAD alone would not establish the version consumers install. [Registry metadata](https://registry.npmjs.org/bitecs), [package manifest](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/package.json), [current documentation](https://bitecs.dev/docs/introduction).

The recommendation is to use **world-owned component stores, ordinary functions for systems, explicit execution phases, and feature-oriented folders with a small shared data vocabulary**. Start with straightforward SoA stores and single-threaded execution. Move to narrower typed storage, spatial indexing, incremental work, or workers when measurements justify them. These organizational choices are recommendations made in this guide; bitECS does not impose a folder structure or provide an engine scheduler.

## Reading map

1. [The paradigm](#the-paradigm)
2. [A complete current-API foundation](#a-complete-current-api-foundation)
3. [Worlds, entity identity, and storage ownership](#worlds-entity-identity-and-storage-ownership)
4. [Components and lifecycle](#components-and-lifecycle)
5. [Queries and safe iteration](#queries-and-safe-iteration)
6. [Systems and simulation timing](#systems-and-simulation-timing)
7. [Observers and external integrations](#observers-and-external-integrations)
8. [Relationships and hierarchies](#relationships-and-hierarchies)
9. [Prefabs](#prefabs)
10. [Serialization, networking, and saves](#serialization-networking-and-saves)
11. [Workers and shared memory](#workers-and-shared-memory)
12. [Folder structure as the game grows](#folder-structure-as-the-game-grows)
13. [Performance and maintainability](#performance-and-maintainability)
14. [Current API inventory](#current-api-inventory)
15. [Documentation discrepancies and production boundaries](#documentation-discrepancies-and-production-boundaries)
16. [Official documentation and example inventory](#official-documentation-and-example-inventory)

Examples below are original teaching examples using the current interfaces, rather than a reproduction of the documentation. The final inventory links to the official snippets, including examples that need caveats. Complete examples are labeled; smaller snippets are identified as continuations or sketches. Merge imports shown in a continuation into the original file's imports instead of declaring the same binding twice. No older API is needed for the patterns below.

## The paradigm

An entity is a numeric handle, not a game object with methods. A component reference identifies a kind of data or a tag. The world tracks which entities possess which references. Your arrays or objects hold the actual values. A query selects entity handles by component membership; a system is simply a function using those handles to read or write data. The library deliberately separates membership from storage. [Introduction](https://bitecs.dev/docs/introduction), [component implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

For example, a character can possess position, velocity, health, a controllable tag, and a relationship to its equipped item. Nothing requires a `Character` superclass. A projectile can reuse position and velocity while adding lifetime and damage. Systems share fields without sharing object inheritance. Entity creation recipes belong in factories; entity behavior belongs in systems.

There are two different forms of change:

- **Structural change:** creating/removing an entity or adding/removing component membership. This updates masks, cached query sets, relationship bookkeeping, and lifecycle observers.
- **Value change:** assigning to an owned store, such as `Position.x[eid]`. This writes data directly. It does not automatically notify `onSet`, change query membership, or update a spatial index.

That distinction is fundamental to performance and correctness. A numeric field changing from 10 to 0 does not remove a `Health` component. A query tests presence, not arbitrary numeric predicates. Your system must filter by value or deliberately change membership. [Component implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts), [query implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

**Storage choice is independent of the ECS model.** SoA keeps a field's values in one array; AoS keeps an entity's fields together in an object. Typed arrays choose a fixed numeric representation. Regular arrays can grow and hold other values. Components can use custom structures, but the serializer only understands its supported formats. None of these choices means component data is automatically initialized or erased by the core. [Storage helpers](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/soa.ts), [serialization implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts).

Do not assume archetype-style storage compaction. Queries maintain dense lists of matching handles, while your component arrays remain indexed according to your storage convention. Adjacent query entries need not address adjacent component slots. Typed arrays give a contiguous numeric backing allocation; ordinary JavaScript arrays leave representation to the runtime. This distinction matters when interpreting locality claims or budgeting for a large highest entity ID. [Query storage](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/SparseSet.ts).

## A complete current-API foundation

Install in a new application with `bun add bitecs@0.4.0`. Keep the lockfile. The package supplies TypeScript declarations and separate core and serialization entry points. The following complete, headless TypeScript example uses world-local stores and can run under Bun. [Installation](https://bitecs.dev/docs/installation), [package exports](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/package.json).

```ts
import {
  addComponents,
  addEntity,
  createWorld,
  query,
} from 'bitecs'

function createGame() {
  const components = {
    Position: { x: [] as number[], y: [] as number[] },
    Velocity: { x: [] as number[], y: [] as number[] },
    Player: {},
  }
  return createWorld({
    components,
    time: { stepSeconds: 1 / 60, tick: 0 },
  })
}

type GameWorld = ReturnType<typeof createGame>

function spawnPlayer(world: GameWorld, x: number, y: number) {
  const { Position, Velocity, Player } = world.components
  const eid = addEntity(world)
  // Initialize owned backing data before membership observers can run.
  Position.x[eid] = x
  Position.y[eid] = y
  Velocity.x[eid] = 3
  Velocity.y[eid] = 0
  addComponents(world, eid, Position, Velocity, Player)
  return eid
}

function movementSystem(world: GameWorld) {
  const { Position, Velocity } = world.components
  const dt = world.time.stepSeconds
  for (const eid of query(world, [Position, Velocity])) {
    Position.x[eid] += Velocity.x[eid] * dt
    Position.y[eid] += Velocity.y[eid] * dt
  }
}

const game = createGame()
const player = spawnPlayer(game, 5, 8)
for (let tick = 0; tick < 60; tick++) {
  movementSystem(game)
  game.time.tick++
}
console.log(game.components.Position.x[player]) // Approximately 8
```

This factory also solves a practical ownership problem: two tests or two game sessions obtain different component objects and arrays. Context inference gives systems a concrete world type. A full `World` annotation would discard useful context information unless given the corresponding context type. The source defines `World<T>` from the supplied context rather than declaring a mandatory world class. [World types and creation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/World.ts).

Regular arrays in this example intentionally require the spawn recipe to initialize every queried field. Typed arrays begin at zero, but a recycled slot can still contain values from its previous occupant. A useful project invariant is: **every newly attached data component is fully initialized before the next simulation phase**. If `onAdd` consumers need data immediately, use a deliberate initialization protocol; see the observer section.

## Worlds, entity identity, and storage ownership

`createWorld(context)` augments and returns the supplied object. Resources such as time, input snapshots, seeded random state, command queues, renderer handles, physics handles, asset catalogs, or networking state can live in that context. bitECS bookkeeping lives behind an internal symbol. Keep external resources owned by a game/session factory with explicit disposal. `deleteWorld` deletes ECS internals; it does not destroy a renderer, terminate a worker, or clear your arrays. [World implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/World.ts).

World membership state is separate, but component values are only isolated if their storage is isolated. A module-global `Position` store shared by two default worlds can be overwritten because both worlds allocate the same numeric IDs. Either create stores separately per world, as above, or intentionally share an entity index whenever sharing stores. [World guide](https://bitecs.dev/docs/world), [entity index implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/EntityIndex.ts).

Complete example of the latter:

```ts
import { addEntity, createEntityIndex, createWorld } from 'bitecs'

const ids = createEntityIndex()
const Position = { x: [] as number[], y: [] as number[] }
const firstMatch = createWorld({ components: { Position } }, ids)
const secondMatch = createWorld({ components: { Position } }, ids)

const a = addEntity(firstMatch)
const b = addEntity(secondMatch)
console.assert(a !== b)
```

**Recommendation:** use distinct stores and distinct indices for independent matches, editor previews, replay simulations, and tests. A shared index is advanced shared ownership: in 0.4.0 `entityExists` checks index liveness, rather than independent membership in each world. Do not interpret a successful liveness check as evidence that a handle belongs to a particular session. Keep game/session identity at your API boundary. [Entity implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Entity.ts).

IDs start at 1, leaving 0 useful for “no entity.” Removed IDs can be immediately recycled. Without versioning, a saved handle may refer to a different entity after reuse, so checking `entityExists` alone cannot detect that case. Persistent save IDs, network IDs, and editor selection IDs should not be silently treated as interchangeable with local ECS handles. [Entity guide](https://bitecs.dev/docs/entity).

`createEntityIndex(withVersioning(bits))` embeds a generation in the handle. `getId(index, handle)` extracts its base slot; `getVersion(index, handle)` extracts the generation. More generation bits leave fewer slot bits; generations eventually wrap. Versioned handles can become very large, and bitwise encoding has signed-number implications. Core bookkeeping and queries still use the full handle; manually indexing only your data by the base slot is a separate storage convention to apply consistently, not a global fix for internal allocation. Profile and test this mode before adopting it for a large, typed-array-based game. [Versioning implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/EntityIndex.ts), [hierarchy allocation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Hierarchy.ts).

`resetWorld` in the inspected implementation replaces registrations, query caches, hierarchy data, and the entity index; it leaves custom context and component stores alone. This differs from the site's “keep registrations” description. It also discards versioning/shared-index configuration. Prefer rebuilding a session through its factory, or explicitly rebuilding registrations, observers, serializer state, and resource state after reset. [World implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/World.ts).

## Components and lifecycle

Component identity is reference identity. Reconstructing `{}` for a tag or reconstructing a position object creates a different component. Preserve references throughout a session; factories create them once at session startup. `addComponent`, `addComponents`, `removeComponent`/`removeComponents`, and `hasComponent` manage membership. Registration usually happens automatically; explicit registration is useful for setup, but register a given reference only once. The implementation does not treat repeated registration as a harmless idempotent operation. [Component guide](https://bitecs.dev/docs/component), [registration and membership code](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

### Choosing a store

| Storage | Good fit | Cost or invariant |
| --- | --- | --- |
| SoA regular arrays | Initial implementation, flexible numeric fields, growing games | Initialize each field; sparse slots and high IDs need attention |
| SoA typed arrays | Hot numeric simulation, fixed budgets, GPU/worker boundaries | Fixed length and numeric precision/range; explicit capacity policy |
| AoS object arrays | Inventories, dialogue, infrequent nested data | Allocations, object lifetime, and serializer schema need deliberate ownership |
| Tags | Membership/state that materially changes which systems should run | Membership changes maintain queries; toggling thousands each frame has a cost |
| Maps/custom stores | External objects and irregular data | Your lifetime/access rules; raw serializers do not automatically understand them |

These are design recommendations grounded in the library's agnostic storage model, not universal speed rankings. JavaScript engine, entity distribution, access pattern, and workload determine the result. [Introduction and storage discussion](https://bitecs.dev/docs/introduction), [storage helper source](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/soa.ts).

A store factory for a fixed budget can be this small:

```ts
// Complete store definition; not a world-capacity setting.
const maxLiveSlot = 50_000
const Position = {
  x: new Float32Array(maxLiveSlot + 1),
  y: new Float32Array(maxLiveSlot + 1),
}
```

The extra slot accounts for IDs starting at 1. The library does not automatically cap entity creation to that length. Typed-array out-of-range writes do not expand storage. Enforce an allocation policy before writing, and remember that “maximum simultaneously alive” and “largest handle used as an index” can differ. Float32 changes numeric precision; unsigned integer arrays wrap values to their representable range. [Entity allocation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/EntityIndex.ts), [component storage examples](https://bitecs.dev/docs/component).

`soa(spec)` is an identity convenience. `aos<T>(spec?)` creates an array and optionally attaches field-schema metadata; neither transforms storage into a new engine-managed format. Plain arrays and objects remain valid. [Helpers](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/soa.ts).

Removing membership or an entity does **not** erase your user-owned data. Initialize recycled slots, release external object references, and avoid retaining stale values in serializers. For expensive resources, keep a cleanup table keyed by entity and dispose its entries during an explicit destruction phase or lifecycle integration. Queries should determine membership; an old nonzero array value is not evidence that a component still exists. [Removal implementations](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Entity.ts), [component removal](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

## Queries and safe iteration

`query(world, [A, B])` selects entities with both references. `And`/`All`, `Or`/`Any`, and `Not`/`None` describe membership constraints. `Not(A, B)` excludes entities possessing either excluded reference. The current implementation compiles simple operators into masks; it does not implement an arbitrary boolean expression tree. In particular, do not assume nested operators or multiple independent `Or` groups have ordinary nested boolean semantics. [Query guide](https://bitecs.dev/docs/query), [query compilation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

Continuation of the foundation example:

```ts
import { Not, Or, query } from 'bitecs'

const { Position, Velocity, Player } = game.components
const Frozen = {}
const AIControlled = {}

const movable = query(game, [Position, Velocity, Not(Frozen)])
const controlled = query(game, [Position, Or(Player, AIControlled)])
// Numeric filters remain ordinary application logic.
for (const eid of movable) {
  if (Position.y[eid] < -100) {
    // Apply an application-specific out-of-bounds policy.
  }
}
```

Query results are cached mutable collections exposed with readonly TypeScript types. Treat them as borrowed views, not permanent snapshots. Structural operations update membership; removals are queued until a normal query call or `commitRemovals(world)`. A component/entity is already logically removed before the cached list is compacted. Adds are not similarly deferred. Sparse-set removal swaps entries, so ordinary iteration order is not a stable gameplay order. [Query updates](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts), [sparse-set implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/SparseSet.ts).

A nested normal query can commit pending removals and mutate the outer result during iteration. Use `isNested`/`noCommit` on inner queries, or `{ commit: false }`; do not call `commitRemovals` until active borrowed iterations end. This prevents compaction but does not make removed entities valid again. When a nested pass can encounter entities just removed by the outer pass, check liveness/membership or, preferably, defer all structural changes to a controlled phase. [Query options and mutation behavior](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

Pattern sketch, assuming these stores belong to one world:

```ts
import { entityExists, isNested, query } from 'bitecs'

for (const source of query(world, [Position, Sensor])) {
  for (const target of query(world, [Position, Detectable], isNested)) {
    if (!entityExists(world, target)) continue
    // Sense now; queue structural consequences for a later phase.
  }
}
```

For a stable snapshot, explicitly copy once at an appropriate boundary: `Array.from(query(world, terms))`. Do not add this allocation to every hot system by default. For scalable sensing/collisions, a broad-phase spatial index is usually more consequential than the exact nested-query spelling.

`asBuffer` requests a `Uint32Array` query result; `{ buffered: true }` is an options-object alternative. `isNested` can be combined with `asBuffer`. Keep one representation per query shape: conversion from an existing normal cache to buffered registration has costs, and subsequent calls may expose the cached buffer even without the modifier. Prefer the modifier form consistently and avoid mixing modifier objects with options objects. The implementation chooses the modifier path when any modifier is supplied. [Query implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

Queries are cached by component/operator shape. A stable `[Position, Velocity]` is cheap to reuse compared with continuously inventing new component references or target-specific queries. Initial registration scans entities; membership changes update relevant registered queries. `registerQuery` and `removeQuery` exist but are low-level tools; see the production-boundary table before relying on query teardown in a long-running editor. [Query registration and removal](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

## Systems and simulation timing

Keep execution order visible in one composition module. A practical sequence is input snapshot → decisions → forces/velocity → movement → collision resolution → combat → lifecycle commands → presentation extraction. The correct order depends on the game; document who reads/writes shared components and when newly spawned entities become eligible. bitECS supplies neither automatic dependency analysis nor a deterministic scheduler. [System guide](https://bitecs.dev/docs/system), [core exports](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/index.ts).

Ordinary functions may return `void`; the exported `pipe` helper composes return values, so world pipelines using it must return their world. An explicit array loop avoids that return convention and makes instrumentation easy. [Pipeline helper](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/pipe.ts).

Continuation of the foundation example:

```ts
const simulationSystems: Array<(world: GameWorld) => void> = [
  movementSystem,
]

function stepGame(world: GameWorld) {
  for (const system of simulationSystems) system(world)
  world.time.tick++
}
```

Use seconds consistently. For replayable simulation, drive `stepGame` with fixed ticks and inject input/RNG state; presentation can interpolate previous/current values. Fixed timesteps alone do not guarantee determinism: iteration order, random sources, floating-point differences, async completion, and external physics matter. A bounded accumulator also prevents a suspended tab from trying to simulate minutes of missed frames. This is an architectural recommendation, not a bitECS API requirement.

Browser integration sketch, using the existing `game` and `stepGame`:

```ts
const fixedSeconds = game.time.stepSeconds
let previousMs = performance.now()
let remainder = 0

function frame(nowMs: number) {
  remainder += Math.min((nowMs - previousMs) / 1000, 0.1)
  previousMs = nowMs
  let steps = 0
  while (remainder >= fixedSeconds && steps < 5) {
    stepGame(game)
    remainder -= fixedSeconds
    steps++
  }
  if (steps === 5) remainder = Math.min(remainder, fixedSeconds)
  // Render with interpolation factor remainder / fixedSeconds.
  requestAnimationFrame(frame)
}
requestAnimationFrame(frame)
```

The clamp and step limit are deliberate game-time policies; choose them for your game's requirements. For an authoritative simulation, a server must make a different overload decision than an ornamental browser animation. Under Bun, a headless loop may sleep between ticks, but a timer's precision does not guarantee a simulation's deadline.

### A controlled lifecycle boundary

Removal compaction and ID reuse are different events: an ID can be recycled immediately even while an older query view still contains it. A practical lifecycle phase finishes all active iterations, processes removals, commits them, then processes spawns. Capture any death-effect data before removal. The following continuation of the foundation example assumes commands were collected during the current tick and no other code allocates entities before this phase:

```ts
import { commitRemovals, entityExists, removeEntity } from 'bitecs'

type LifecycleCommand =
  | { type: 'despawn'; eid: number }
  | { type: 'spawnPlayer'; x: number; y: number }

function applyLifecycle(world: GameWorld, commands: LifecycleCommand[]) {
  const removed = new Set<number>()
  for (const command of commands) {
    if (command.type !== 'despawn' || removed.has(command.eid)) continue
    if (entityExists(world, command.eid)) {
      // Release external resources and capture effects before this removal.
      removeEntity(world, command.eid)
      removed.add(command.eid)
    }
  }
  commitRemovals(world)
  for (const command of commands) {
    if (command.type === 'spawnPlayer') {
      spawnPlayer(world, command.x, command.y)
    }
  }
  commands.length = 0
}
```

Keep the command buffer on the session/world and define its ownership; this example accepts it explicitly to keep the original foundation unchanged. Queues delayed across arbitrary future ticks need a stronger identity/generation policy. The `Set` is a clarity-first deduplication choice; use reusable storage only if measured lifecycle bursts require it. This phase policy is an application recommendation, not a built-in command-buffer feature. [Immediate recycling and deferred query removal](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Entity.ts).

## Observers and external integrations

`observe` installs a callback and returns an unsubscribe function. `onAdd`/`onRemove` observe transitions into/out of a query pattern; they are not generic property-write subscriptions. `onSet` is invoked by the setter pathway, and `onGet` supplies values for `getComponent`. Use them for initialization/access contracts and bridges to rendering, audio, physics, or diagnostics. Keep simulation ordering visible. [Observer guide](https://bitecs.dev/docs/observers), [observer dispatch](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts).

Complete example showing the explicit setter/getter contract:

```ts
import {
  addComponent, addEntity, createWorld, getComponent,
  observe, onGet, onSet, set, setComponent,
} from 'bitecs'

const world = createWorld()
const Shield = { points: [] as number[] }
type ShieldInput = { points: number }

const stopSet = observe(world, onSet(Shield),
  (eid, value: ShieldInput) => {
    Shield.points[eid] = Math.max(0, value.points)
  })
const stopGet = observe(world, onGet(Shield), eid => ({
  points: Shield.points[eid],
}))

const hero = addEntity(world)
addComponent(world, hero, set(Shield, { points: 30 }))
setComponent(world, hero, Shield, { points: 12 })
console.log(getComponent(world, hero, Shield)) // { points: 12 }
stopGet()
stopSet()
```

`set` packages a reference and payload; the callback writes the storage. It does not infer how to update any arbitrary custom store. Likewise, `getComponent` does not automatically reconstruct a row from SoA. With a present registered component and no getter observer, the inspected implementation returns an empty object. Getter observers are aggregated by shallow-merging returned objects; primitive callback returns do not create a scalar getter contract. Direct array writes bypass validation and notifications, so define which pathways a feature uses. Core component/payload typing is permissive; typed feature functions such as `setShield(world, eid, input)` can enforce the public contract. [Component data access](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts), [observable aggregation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/Observer.ts).

An important ordering detail: a newly added component triggers membership observers **before** its `set` payload is applied. A renderer's `onAdd(Sprite)` callback must not assume `set(Sprite, payload)` has already written `textureId`. Either use `onSet` to update external state, initialize a separate backing resource before attachment with documented rules, or create/update presentation objects during an explicit synchronization phase. [Add sequence](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

Callbacks run synchronously inside ECS operations. Structural mutations inside callbacks can cause re-entrancy and hidden order dependencies. Queue effects and drain at a chosen phase; capture values needed after removal, because a queued numeric handle can be recycled. The following continuation uses a queue for external effects, not deferred mutation of a stale handle:

```ts
import { observe, onRemove } from 'bitecs'

const effects: Array<{ x: number; y: number }> = []
const stopEffects = observe(world, onRemove(Shield), eid => {
  // Capture any required values now, rather than retaining eid forever.
  effects.push({ x: 0, y: 0 })
})

function drainEffects(emit: (x: number, y: number) => void) {
  for (const effect of effects) emit(effect.x, effect.y)
  effects.length = 0
}
// Call stopEffects() when this integration is disposed.
```

Do not translate `onRemove(Health)` directly into “an entity died”: the component may have been removed during a transformation, scene teardown, or network correction. Model death as a domain transition owned by combat. For high-volume effects, replace per-effect objects with a reusable bounded buffer if profiling identifies garbage collection as a bottleneck.

## Relationships and hierarchies

A relationship connects a subject to a target. `Owns(item)` is a target-specific component placed on the owner; `Owns(item).quantity[owner]` is per-subject data in that target's store. The relation factory caches pairs, so repeated calls with the same target yield the same component reference. Every distinct target can create another component/store, which is relevant to high-churn or dense graphs. [Relationship guide](https://bitecs.dev/docs/relationships), [pair storage](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Relation.ts).

Complete example:

```ts
import {
  addComponent, addEntity, createRelation, createWorld,
  getRelationTargets, makeExclusive, query, Wildcard, withStore,
} from 'bitecs'

const world = createWorld()
const Equipped = createRelation(makeExclusive)
const Owns = createRelation(
  withStore(() => ({ quantity: [] as number[] })),
)
const hero = addEntity(world)
const bow = addEntity(world)
const arrows = addEntity(world)

addComponent(world, hero, Equipped(bow))
addComponent(world, hero, Owns(arrows))
Owns(arrows).quantity[hero] = 12

const bowUsers = query(world, [Equipped(bow)])
const equippedSubjects = query(world, [Equipped(Wildcard)])
const equippedTargets = query(world, [Wildcard(Equipped)])
const ownedItems = getRelationTargets(world, hero, Owns)
```

`Relation('*')` and `Relation(Wildcard)` match subjects having any target. `Wildcard(Relation)` selects targets participating in that relation. `Wildcard(targetEid)` selects subjects pointing at a target across relation types. `Pair(relation, target)` is an alternative spelling of `relation(target)`. [Wildcard behavior](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Relation.ts), [pair bookkeeping](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

| Relation option | Meaning | Suggested application |
| --- | --- | --- |
| `withStore(factory)` / `store` | A distinct store for each target pair | Quantity held, relationship weight, equipped slot |
| `makeExclusive` / `exclusive` | A subject has at most one target of this relation | Current target, parent, selected weapon |
| `withAutoRemoveSubject` / `autoRemoveSubject` | Delete subjects when the target is deleted | Child effects with an owner's lifetime |
| `withOnTargetRemoved(callback)` / `onTargetRemoved` | Callback when a linked target disappears | Queue domain-specific cleanup or replacement |

These can be supplied through modifiers or a single options object. Configure relations before creating pairs. In 0.4.0, the target-removal callback's exported TypeScript type disagrees with its invocation; use an isolated, verified adapter before depending on this feature. [Relation configuration](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Relation.ts), [target-removal call site](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Entity.ts).

Use auto-removal only for genuine lifetime ownership. A “targets enemy” relationship should normally lose its target when the enemy dies, not delete the attacker. For a high-frequency changing numeric target that is not queried relationally, an entity-reference field may be simpler and cheaper; then the application owns validation/cleanup. This is a modeling recommendation.

### Hierarchies

`Hierarchy(relation)` and its alias `Cascade` arrange results by depth, and `getHierarchyDepth`/`getMaxHierarchyDepth` inspect tracked depth. For a tree, define an exclusive parent relation and validate against cycles at the command boundary. Avoid treating an arbitrary cyclic/multi-parent graph as a tree merely because a traversal API accepts it. [Hierarchy implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Hierarchy.ts).

Complete small hierarchy example:

```ts
import {
  addComponent, addEntity, createRelation, createWorld,
  getHierarchyDepth, Hierarchy, makeExclusive, query,
} from 'bitecs'

const world = createWorld()
const ChildOf = createRelation(makeExclusive)
const Transform = { localX: [] as number[], worldX: [] as number[] }
const root = addEntity(world)
const child = addEntity(world)
for (const eid of [root, child]) {
  addComponent(world, eid, Transform)
  Transform.localX[eid] = eid === root ? 10 : 2
  Transform.worldX[eid] = 0
}
addComponent(world, child, ChildOf(root))

const ordered = query(world, [Transform, Hierarchy(ChildOf)])
console.log([...ordered], getHierarchyDepth(world, child, ChildOf))
```

Hierarchy support has important 0.4.0 boundaries. A supplied numeric `depth` selects exactly that depth, rather than a traversal depth limit; the depth-specific code path ignores additional component filters. Hierarchy calls also take a different removal-commit path, and cached ordering/invalidation deserves explicit tests for deletion, reparenting, and new members. A published-package probe reproduced stale descendant depths after attaching one existing multi-level root beneath another tree, matching [issue #214](https://github.com/NateTheGreatt/bitECS/issues/214). Keep authoritative transform propagation behind a small module so an application-owned traversal can replace it if required. [Hierarchy query dispatch](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts), [hierarchy caches](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Hierarchy.ts).

## Prefabs

`addPrefab` creates an entity marked with `Prefab`; ordinary queries exclude it. In the published implementation, even explicitly querying `[Prefab]` does not enumerate templates; tooling can instead inspect `getAllEntities(world)` and filter with `hasComponent(world, eid, Prefab)`. Adding `IsA(template)` to an entity inherits component membership and uses `onGet`/`onSet` to transfer values. Multi-level inheritance exists, but mutable state should still have explicit ownership. [Prefab guide](https://bitecs.dev/docs/prefabs), [inheritance implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts).

Complete copy-at-spawn example:

```ts
import {
  addComponent, addEntity, addPrefab, createWorld,
  IsA, observe, onGet, onSet, set,
} from 'bitecs'

const world = createWorld()
const Stamina = { current: [] as number[], maximum: [] as number[] }
observe(world, onGet(Stamina), eid => ({
  current: Stamina.current[eid],
  maximum: Stamina.maximum[eid],
}))
observe(world, onSet(Stamina),
  (eid, value: { current: number; maximum: number }) => {
    Stamina.current[eid] = value.current
    Stamina.maximum[eid] = value.maximum
  })

const scoutTemplate = addPrefab(world)
addComponent(world, scoutTemplate,
  set(Stamina, { current: 75, maximum: 75 }))
const scout = addEntity(world)
addComponent(world, scout, IsA(scoutTemplate))
console.assert(Stamina.current[scout] === 75)
```

The inspected implementation copies inherited data via getters/setters when `IsA` is added. It does not establish automatic live linkage of primitive values. The site's example gallery claims later template changes affect all instances; do not rely on that without implementing such behavior explicitly. Shallow object-returning getters/setters can accidentally share nested arrays; clone mutable inventory or use immutable asset/config references deliberately. [Recursive inheritance](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts), [official prefab example](https://bitecs.dev/examples/#prefabs).

For a growing game, ordinary typed spawn functions plus immutable authored data are often easier to trace than deep prefab inheritance. Use the built-in prefab feature where the inherited composition helps; put it behind factories such as `spawnScout` so callers do not depend on its initialization mechanics. Avoid one prefab layer per conceptual noun.

## Serialization, networking, and saves

Import serializers from `bitecs/serialization`, separately from core. Choose a layer based on what must be transferred:

| Layer | Captures | What the application still owns |
| --- | --- | --- |
| SoA | Supported component field values for selected IDs | Membership, removed slots, packet schema, ID mapping |
| AoS | Values in schema-described row/object storage | Same responsibilities; schema must describe object fields |
| Observer | Tagged-entity/component lifecycle operations; relation operations/data in supported forms | Regular value updates, initial bootstrap, transport reliability |
| Snapshot | Selected world structure plus supported values | Replacement policy, assets/resources, versioned save format, migration |

The official guide describes these layers. Exact contracts and constraints below are from the published serializer source; they matter more than the phrase “full world state.” [Serialization guide](https://bitecs.dev/docs/serialization), [serializer exports](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/index.ts).

### Data formats, tags, and mappings

Numeric typed arrays carry their width inherently. Regular arrays can be marked with `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`, or `f64`. These markers specify binary encoding; `f32([])` remains a regular array, not a `Float32Array`. Unmarked numeric arrays use Float64 encoding. `str` enables UTF-8 strings, `array(type)` describes per-entity arrays, and nesting `array` describes nested arrays. `ref` marks entity references for mapping during decode; an ordinary `u32` is only a number and is not automatically remapped. [Type tags and nested-array support](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts).

Complete standalone raw-store example, demonstrating different source/destination IDs:

```ts
import {
  array, createSoADeserializer, createSoASerializer,
  f32, ref, str, u16,
} from 'bitecs/serialization'

const outgoing = {
  x: f32(),
  owner: ref(),
  title: str(),
  items: array(u16),
}
const incoming = {
  x: f32(),
  owner: ref(),
  title: str(),
  items: array(u16),
}
outgoing.x[1] = 1.25
outgoing.owner[1] = 2
outgoing.title[1] = 'Courier'
outgoing.items[1] = [10, 11]

const encode = createSoASerializer([outgoing], {
  buffer: new ArrayBuffer(4096),
})
const decode = createSoADeserializer([incoming])
const map = new Map([[1, 101], [2, 102]])
decode(encode([1]), map)
console.assert(incoming.x[101] === 1.25)
console.assert(incoming.owner[101] === 102)
```

Pass the same component/schema ordering to encoder and decoder. The packet contains numeric component/field indices, not a self-describing TypeScript schema. Changing component order or fields can change interpretation. Wrap the payload with an application protocol/save-format version and validate compatibility before decoding. Keep serializable schemas in one module shared by server/client or writer/reader. [SoA wire-format implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts).

### Diffs and memory budgets

SoA and AoS support `{ diff: true, epsilon, buffer }` on encoders; configure matching `{ diff: true }` on decoders. Diffs compare against an encoder-owned shadow of earlier values. SoA initializes its shadows with zeros, so its first delta does not guarantee a full baseline containing every zero-valued field. Its floating shadows also advance when a change is below epsilon, so a series of small per-call changes can remain unsent; epsilon is not an accumulated-error guarantee. AoS uses a different shadow/update mechanism. The default floating epsilon is `0.0001`; integers are compared without floating tolerance. Sending fewer bytes still requires comparison work and shadow memory. The default buffer for these serializers, observer encoding, and snapshots is **100 MiB per encoder** (`1024 * 1024 * 100` bytes), not a dynamically budgeted packet pool. Supply a measured bounded buffer; overflow is not automatically turned into multiple packets. Encoding returns a sliced buffer, so output allocation/copying remains. [SoA implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts), [AoS implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/AoSSerializer.ts), [observer implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/ObserverSerializer.ts).

Continuation of the raw-store example:

```ts
const encodeDelta = createSoASerializer([outgoing], {
  diff: true,
  epsilon: 0.001,
  buffer: new ArrayBuffer(4096),
})
const decodeDelta = createSoADeserializer([incoming], { diff: true })
decodeDelta(encodeDelta([1]), map) // Establish this encoder's baseline.
outgoing.x[1] += 0.5
decodeDelta(encodeDelta([1]), map)
```

**Networking recommendation:** an encoder's baseline represents its own send history, not acknowledged receipt at every client. Do not blindly share one stateful diff encoder among independently connected clients. Reliable ordered delivery, per-client baselines, sequence numbers, resynchronization, bootstrap snapshots, and loss policy belong to the application. A newly connected peer needs full baseline state. Restart/rebootstrap appropriately when IDs are recycled. In published 0.4.0, a recycled ID with unchanged values can produce a zero-byte delta because its encoder shadow still belongs to the old occupant; this was reproduced during this research and is tracked in [issue #209](https://github.com/NateTheGreatt/bitECS/issues/209). Proposed reset/removal APIs shown in that issue are **not released APIs** and are intentionally not taught here. Diffs are a compression tool, not a replication protocol.

For arrays/objects, test actual mutation patterns. AoS shadows shallow-copy objects, so in-place edits of a shared nested array can escape expected change detection. A recipe that replaces nested immutable values differs from one that mutates them in place. Do not promise efficient arbitrary-object deep diffs. [AoS shadow behavior](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/AoSSerializer.ts).

A second reproduced 0.4.0 issue affects `array(f32)` fields with diff mode: even the first encode can emit no data because the floating comparison operates on arrays. Use full encoding (`diff: false`) for those fields pending a verified fix, or maintain an application-owned encoding strategy. [Issue #213](https://github.com/NateTheGreatt/bitECS/issues/213). The raw-store and snapshot examples in this guide use full encoding; the delta example only changes ordinary numeric/string/reference fields plus an unchanged integer array.

The implementation uses 32-bit change masks for SoA fields and AoS component groups, so keep each diff mask within 32 entries. Snapshot component indices/counts and observer component indices use 8-bit fields. Validate schemas at setup and test worst-case entity/relationship composition rather than silently exceeding the encoding widths. [SoA masks](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts), [AoS masks](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/AoSSerializer.ts), [snapshot counters](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SnapshotSerializer.ts).

### AoS schemas

An array of `{ x, y }` rows is valid core storage, but the AoS serializer needs metadata describing field encodings. `aos` can attach that metadata. Complete example:

```ts
import { aos } from 'bitecs'
import {
  createAoSDeserializer, createAoSSerializer, f32, u8,
} from 'bitecs/serialization'

type Row = { charge: number; tier: number }
const schema = { charge: f32(), tier: u8() }
const source = aos<Row, typeof schema>(schema)
const target = aos<Row, typeof schema>({ charge: f32(), tier: u8() })
source[1] = { charge: 2.5, tier: 3 }
const encode = createAoSSerializer([source], {
  buffer: new ArrayBuffer(1024),
})
const decode = createAoSDeserializer([target])
decode(encode([1]))
console.assert(target[1].charge === 2.5)
```

### Structural replication

Create an observer serializer before the mutations you want to capture. It subscribes to lifecycle transitions, rather than walking existing entities for an automatic initial snapshot. Put a `Networked` tag on the entities to replicate. Send structural operations first, then value data, and use the **same source→local ID map** for both decoders. Unmapped value records otherwise fall back to their source IDs. [Observer encoder/decoder](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/ObserverSerializer.ts), [SoA remapping](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts).

Complete minimal replication example:

```ts
import { addComponents, addEntity, createWorld, query, removeEntity } from 'bitecs'
import {
  createObserverDeserializer, createObserverSerializer,
  createSoADeserializer, createSoASerializer, f32,
} from 'bitecs/serialization'

function createPeer() {
  return {
    world: createWorld(),
    Networked: {},
    Position: { x: f32(), y: f32() },
  }
}
const server = createPeer()
const client = createPeer()
addEntity(client.world) // Local-only entity makes source/local IDs differ.
const ids = new Map<number, number>()
const encodeStructure = createObserverSerializer(
  server.world, server.Networked, [server.Position],
  { buffer: new ArrayBuffer(4096) },
)
const decodeStructure = createObserverDeserializer(
  client.world, client.Networked, [client.Position], { idMap: ids },
)
const encodeValues = createSoASerializer([server.Position], {
  buffer: new ArrayBuffer(4096),
})
const decodeValues = createSoADeserializer([client.Position])

const eid = addEntity(server.world)
addComponents(server.world, eid, server.Networked, server.Position)
server.Position.x[eid] = 10
server.Position.y[eid] = 20

decodeStructure(encodeStructure(), ids)
// The query's public type also permits Uint32Array; the encoder expects an array.
const selected = Array.from(query(
  server.world, [server.Networked, server.Position],
))
decodeValues(encodeValues(selected), ids)
const localEid = ids.get(eid)
if (localEid === undefined) throw new Error('Missing network ID mapping')
console.assert(localEid !== eid)
console.assert(client.Position.x[localEid] === 10)
removeEntity(server.world, eid)
decodeStructure(encodeStructure(), ids)
```

The selected-ID query requires both Networked and Position membership; spawn must also initialize values. The array copy makes the public type boundary explicit; if profiling justifies removing it, isolate a verified conversion/adapter rather than spreading unchecked assertions across gameplay. With multiple heterogeneous components, create per-schema selection groups or explicitly clear removed values and test absent-field semantics: raw value serialization has no world argument and cannot check whether a component is attached. Stale stores are especially hazardous after membership removal. Relation replication also needs focused tests: the 0.4.0 observer implementation tracks wildcard transitions and stores one target per relation index, so arbitrary multi-target updates should not be assumed complete. [Observer relation tracking](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/ObserverSerializer.ts).

### Snapshots and persistence

`createSnapshotSerializer(world, components, buffer?)` produces structure plus supported data; its returned function also accepts selected entity IDs in the implementation. `createSnapshotDeserializer` reconstructs/upserts entities and returns the source→local map. It does not first delete unrelated live entities or remove components omitted by the snapshot. Load into a fresh session or perform an explicit replace policy. Do not assume original numeric IDs are restored. [Snapshot implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SnapshotSerializer.ts).

Complete snapshot roundtrip into fresh stores and a fresh destination world:

```ts
import { addComponent, addEntity, createWorld, hasComponent } from 'bitecs'
import {
  createSnapshotDeserializer, createSnapshotSerializer, f32,
} from 'bitecs/serialization'

const savedWorld = createWorld()
const savedPosition = { x: f32(), y: f32() }
const oldEid = addEntity(savedWorld)
addComponent(savedWorld, oldEid, savedPosition)
savedPosition.x[oldEid] = 4.5
savedPosition.y[oldEid] = 9
const save = createSnapshotSerializer(
  savedWorld, [savedPosition], new ArrayBuffer(4096),
)
const packet = save()

const loadedWorld = createWorld()
const loadedPosition = { x: f32(), y: f32() }
const load = createSnapshotDeserializer(loadedWorld, [loadedPosition])
const mapping = load(packet)
const newEid = mapping.get(oldEid)
if (newEid === undefined) throw new Error('Snapshot omitted the entity')
console.assert(hasComponent(loadedWorld, newEid, loadedPosition))
console.assert(loadedPosition.x[newEid] === 4.5)
```

Create fresh codecs/maps too when starting a replacement load; a persistent decoder map can retain IDs whose destination entities have since been destroyed. Do not confuse a successful first restore with reusable replace-all behavior.

A save must separately cover any world-context state that matters: random generator, time/tick, quest progression, asset identifiers, spatial/index rebuilds, external physics state, and application schema version. Snapshot serialization uses SoA for ordinary component values; it is not a universal serializer for arbitrary Maps, engine objects, or complex AoS state. The relation-data path has narrower numeric/ref handling than the generic string/nested-array codec. Keep compatibility fixtures covering each actual schema.

## Workers and shared memory

bitECS offers compatible building blocks rather than a worker scheduler. Component typed arrays can be backed by `SharedArrayBuffer`; buffered query results can also use shared storage. The sparse-set implementation falls back to `ArrayBuffer` when shared memory is unavailable, so `asBuffer` alone does not prove a zero-copy worker path. [Multithreading guide](https://bitecs.dev/docs/multithreading), [buffer allocation implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/SparseSet.ts).

Recommended ownership model:

1. The coordinator owns all structural mutations and the current query set.
2. At a barrier, it selects work and sends the current view, explicit count, shared component views, tick, and partition ranges.
3. Workers update disjoint output rows, or read from an immutable previous-state buffer and write to another buffer.
4. Workers return commands/results and acknowledge completion.
5. The coordinator applies commands and rebuilds membership before starting the next job or rendering dependent data.

Do not mutate a shared query set while workers read it. Buffered sparse sets can grow into a new backing allocation, and existing views have a fixed length; refresh work views at boundaries. Never treat a view sent at startup as a permanently current query. Do not transfer a `SharedArrayBuffer` in a transfer list. [Buffered sparse-set growth](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/utils/SparseSet.ts).

Complete allocation example; the synchronization protocol is intentionally an application responsibility:

```ts
const slots = 10_001
const x = new Float32Array(
  new SharedArrayBuffer(slots * Float32Array.BYTES_PER_ELEMENT),
)
const y = new Float32Array(
  new SharedArrayBuffer(slots * Float32Array.BYTES_PER_ELEMENT),
)
const sharedPosition = { x, y }
```

For browser deployment, shared memory requires the appropriate secure/cross-origin-isolated environment. Use feature detection and a serial fallback. For shared counters or concurrent command queues, design synchronization with `Atomics`; a shared typed array does not make arbitrary read-modify-write safe. A worker barrier does not protect two workers writing the same value during the same job. [MDN SharedArrayBuffer reference](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer), [MDN Atomics reference](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics).

Parallelism has job dispatch, message, synchronization, memory, and debugging costs. Spatial indexes, fewer allocations, less expensive AI frequency, renderer batching, and better work selection may pay off first. Keep a serial implementation as a reference and compare workers on target hardware.

## Folder structure as the game grows

The official docs explain functions and context but do not prescribe project layout. The structure below is this guide's recommendation. Organize around **feature ownership plus shared simulation primitives**, rather than making every entity type a class or creating enormous global `components/` and `systems/` dumping grounds.

### Small game

Start with a few clear files; do not create an architecture framework before the mechanics exist:

```text
src/game/
  create-game.ts       # World, stores, resources, startup and disposal
  components.ts        # A small game's shared data definitions
  spawn.ts             # Typed construction recipes
  simulate.ts          # Explicit ordered systems and tick entry point
  systems/
    movement.ts
    lifetime.ts
  presentation.ts      # Renderer/audio integration
  game.test.ts
```

### Medium game

Once features acquire independent rules, move ownership together before adding networking, workers, or a plugin framework:

```text
src/game/
  create-game.ts
  world.ts
  schedule.ts
  shared/
    transform.ts
    motion.ts
  features/
    movement.ts
    combat/
      components.ts
      damage-system.ts
      combat.test.ts
    inventory/
      components.ts
      commands.ts
      inventory.test.ts
  spawn/
    player.ts
    enemies.ts
  presentation/
    sync-scene.ts
  content/
    enemies.ts
```

This intermediate layout keeps small features in one file and gives larger features their own boundary. The next structure adds integration and persistence boundaries only when those responsibilities exist.

### Large game

Split where independent responsibilities have appeared:

```text
src/game/
  runtime/
    create-game.ts
    world.ts                    # Shared GameWorld/context types
    schedule.ts                 # Only composition root defines phase order
    commands.ts                 # Typed structural command ownership
    dispose.ts
  shared/
    transform.ts                # Widely shared Position/Rotation store factories
    motion.ts                   # Shared Velocity/Force data
    identity.ts                 # Local handle and stable-ID boundary
    random.ts                   # Injected deterministic random source
  features/
    locomotion/
      components.ts
      systems.ts
      commands.ts
      index.ts                  # Deliberate public feature surface
      locomotion.test.ts
    combat/
      components.ts
      apply-damage.ts
      death-system.ts
      spawn-projectile.ts
      index.ts
      combat.test.ts
    inventory/
      components.ts
      relations.ts
      commands.ts
      index.ts
      inventory.test.ts
    ai/
      components.ts
      perception.ts
      decisions.ts
      index.ts
  content/
    enemies.ts                  # Immutable authored definitions, stable content IDs
    items.ts
    levels/
  spawning/
    create-player.ts            # Cross-feature assembly recipes
    create-enemy.ts
  integration/
    rendering/
      create-renderer.ts
      sync-scene.ts
      extract-frame.ts
    physics/
      create-physics.ts
      sync-bodies.ts
    input/
      collect-input.ts
    audio/
      consume-effects.ts
  persistence/
    schema.ts
    save-game.ts
    load-game.ts
    migrations/
  networking/
    protocol.ts
    replication.ts
    interpolation.ts
  workers/
    protocol.ts
    worker-pool.ts
    movement.worker.ts
  diagnostics/
    metrics.ts
    inspect-entity.ts
  tests/
    fixtures/
    replay.test.ts
```

These directories represent roles, not a requirement to create every folder now. A feature may stay in one file until splitting makes navigation easier. Promote a component into `shared` only when unrelated features truly need it. Do not import every internal file through barrels that introduce runtime cycles; public `index.ts` modules should expose a deliberate small surface.

### Dependencies and data ownership

Recommended import direction:

```text
application/session factory
  -> schedule, feature installers, external adapters, content
features
  -> shared data vocabulary, narrow ports, their own implementation
external adapters
  -> shared data contracts and read models
shared
  -> bitECS/types and basic utilities
```

The composition root wires cross-feature behavior. Combat may publish damage/death facts and locomotion may consume an explicit disabled/movement state; neither needs to import the other's entire scheduler. For shared fields, define the write contract: for example AI/input write intent, steering writes acceleration, integration writes position, rendering reads extracted transforms. Multiple writers are acceptable when deliberately ordered, but accidental competing writers are difficult to debug.

Store **mutable simulation state in the world or components**, including cooldowns, accumulated time, queued commands, and RNG state. Avoid module-level timers/counters that leak across sessions/tests. Immutable content tables can be module-level. Feature factories can capture references owned by one session; the factory/disposer must then have the same lifetime as that session.

Keep an entity recipe separate from its runtime features. `spawnEnemy` composes health, motion, sprite, faction, and AI. The movement system should not care whether an entity came from `spawnEnemy`, a save, an editor, or a network packet. Attach the data required by each system, and make factories enforce those invariants.

A narrow feature surface can expose `createStores`, `installObservers`, `spawn`, `applyCommand`, `step`, and `dispose` only where needed. Do not mirror every bitECS function behind a generic wrapper layer. Wrap policies that need ownership: capacity, initialization, ID boundaries, destruction, serialization schemas, or the specific unstable features identified above.

### Growth decisions

| Trigger | Useful next change | Avoid |
| --- | --- | --- |
| More features share one component | Give that data a clear shared owner and writer contract | Importing a feature's internals from everywhere |
| Spawn recipes duplicate invariants | Centralize recipes or immutable definitions | Deep inheritance trees as a default |
| Test worlds interfere | Create stores/resources per world | Resetting only entity membership and hoping globals cleared |
| Hundreds of files become hard to navigate | Group by feature and add a small public entry point | One global folder per technical noun |
| Renderer/physics lifecycle becomes tangled | Session-scoped adapters with disposal and explicit sync | Gameplay mutations hidden in external callbacks |
| Saves/networking become requirements | Dedicated schemas, stable content IDs, migration policy | Treating runtime arrays as a permanent file format |
| Performance hotspots appear | Profile and optimize that feature's data path | Broad rewrites based on ECS folklore |

### Tests that carry their weight

Use headless world factories. Test observable outcomes: movement after a fixed number of ticks; accumulated damage and one destruction; inventory ownership after a target disappears; independent worlds; save/restore with remapped references; late client bootstrap; ID reuse and slot initialization; hierarchy reparent/delete if using it. A compact replay fixture with seed and tick-stamped commands can exercise system order and make failures reproducible.

Keep tests at public feature or simulation boundaries. Avoid asserting every internal array assignment or duplicating implementation branches. A small compatibility suite for version-specific query/observer/serialization behavior is especially valuable here because source and website claims diverge in several places.

## Performance and maintainability

### Measure the game, not the library slogan

The library's performance claims do not establish your game's entity capacity or frame budget. Record at least total frame/tick duration, per-system time, entity counts, structural operations, query shapes, allocation/GC behavior, draw calls, physics work, serializer time/bytes, and memory. Measure representative entity mixes and burst workloads on target runtimes/hardware; regular arrays and typed arrays behave differently across engines. [Official storage/performance discussion](https://bitecs.dev/docs/introduction), [system profiling guidance](https://bitecs.dev/docs/system).

Warm up before measuring steady-state loops. Compare median and tail times, include spawn/death storms and level transitions, and distinguish rendering limits from simulation limits. A fast average that stalls on every wave spawn is not the same as a stable game. Maintain a reproducible workload and document engine, machine, entity count, data layout, and enabled features alongside results.

### Optimize in this order

1. **Remove unnecessary work.** Cull out-of-view presentation; run expensive AI/perception at an appropriate cadence; query only needed entities; share results across a phase when membership is stable.
2. **Fix algorithmic scale.** All-pairs collision/sensing/flocking is quadratic. Introduce a grid, spatial hash, sweep, or other workload-appropriate broad phase. The official pairwise snippets teach mechanics, not a scalable broad phase.
3. **Reduce hot-loop allocation.** Avoid `.map`/`.filter`, per-entity object snapshots, repeated `getComponent` object construction, transient vectors, or copied query results in measured hot paths. Reuse bounded scratch storage when needed.
4. **Choose numeric layout and precision.** Separate hot numeric data from cold authored/config data. Use the smallest representation that preserves required precision and range; calculate component memory from bytes per slot times slot count, then include indices/query caches/shadows/external resources.
5. **Control structural churn and query count.** A membership mutation touches registered query structures. Tags are useful, but a continuously changing scalar is often a field. Avoid generating a fresh component type per transient concept.
6. **Budget serialization.** Select the actual replicated subset, separate different component populations, choose update frequency and precision, retain correct baselines, bound buffers, and measure encoding plus transport costs.
7. **Parallelize independent expensive work.** Only after the job size offsets dispatch/barrier costs and ownership is unambiguous.

These are engineering recommendations derived from the source's update and storage mechanisms, not published benchmark results. [Query mutation costs](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts), [pair caches](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Relation.ts), [serializer shadowing](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/SoASerializer.ts).

### Specific patterns worth keeping

- **Bounded lifecycle phase:** collect spawn/destruction requests and apply them predictably. Initialize all values on allocation, release all owned resources on destruction, and define when new entities enter simulation.
- **Hot/cold split:** transforms/velocity/timers in numeric stores; dialogue, immutable content, textures, and complex inventory descriptions elsewhere with IDs/references.
- **Derived-data policy:** if positions change directly in a system, update bounds/spatial index in an explicit later phase. An `onSet` hook will not notice those direct writes.
- **Read-model boundary:** render/audio/debugging consume data or extracted presentation commands; their callbacks should not quietly own gameplay rules.
- **Stable query shapes:** keep component reference identity stable and avoid building unlimited target-specific cached queries for transient entities.
- **Predictable order:** if gameplay needs order, make that order explicit (stable domain sequence or sorted work list); sparse-set order is not a deterministic priority rule.
- **Session teardown:** stop listeners/observers, terminate workers, release engine handles, discard serializers and query views, then discard stores/world as one owned unit.

### Trade-offs that deserve an explicit decision

| Decision | Performance benefit | Maintenance risk | Suggested default |
| --- | --- | --- | --- |
| Direct store access | Low abstraction overhead | Bypasses validation/observers | Use inside owned systems; typed feature APIs at boundaries |
| Global component singletons | Short imports | World/test collisions and retained data | World-local stores |
| Many state tags | Selective queries | Structural churn and transition invariants | Tags for meaningful membership; scalar for rapidly changing values |
| Relations for every reference | Rich queries and cleanup | Pair/store/query cardinality | Relations when relational queries/lifetime help |
| Deep prefab trees | Reused composition | Initialization precedence and mutable aliasing | Shallow templates or ordinary spawn recipes |
| Fine-grained systems | Easier profiling/ownership | More passes and scheduling complexity | Cohesive responsibility; merge only measured expensive passes |
| Shared-memory workers | Parallel numeric computation | Races, barriers, deployment constraints | Single-threaded first |
| Stateful delta encoders | Lower payload volume | Baseline correctness and per-client state | Full-state bootstrap plus explicit baseline lifecycle |

The goal is a small set of visible invariants that make both correctness and optimization local: what a component means, who owns its values, when it can change, which phase consumes it, and how its lifetime ends.

## Current API inventory

This inventory covers the actual public runtime exports of the current package's modern entry points, including useful exports missing from the website API page. It intentionally excludes the separately exported compatibility entry point. Names alone should not be treated as a recommendation to use every low-level facility. [Core export file](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/index.ts), [serialization export file](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/index.ts).

### Core runtime exports: `bitecs`

| Category | Public values | Purpose/notes |
| --- | --- | --- |
| World | `createWorld`, `resetWorld`, `deleteWorld`, `getWorldComponents`, `getAllEntities` | Create/context, lifecycle, inspection; reset/disposal caveats above |
| Entity | `addEntity`, `removeEntity`, `getEntityComponents`, `entityExists` | Numeric handle lifecycle and component inspection |
| Entity index | `createEntityIndex`, `withVersioning`, `getId`, `getVersion` | Shared IDs and versioned handles |
| Components | `registerComponent`, `registerComponents`, `hasComponent`, `addComponent`, `addComponents`, `removeComponent`, `removeComponents` | Registration/membership; plural remove is an alias |
| Data access | `getComponent`, `setComponent`, `set` | Explicit getter/setter hooks, rather than automatic row access |
| Query lifecycle | `query`, `registerQuery`, `removeQuery`, `commitRemovals` | Cached membership selection and deferred compaction |
| Logical terms | `And`, `All`, `Or`, `Any`, `Not`, `None` | Alias pairs; simple-operator limits above |
| Query modifiers | `asBuffer`, `isNested`, `noCommit` | Buffered IDs and no-commit alias pair |
| Observers | `observe`, `onAdd`, `onRemove`, `onSet`, `onGet` | Subscriptions to membership/data pathways |
| Relationships | `createRelation`, `withStore`, `makeExclusive`, `withAutoRemoveSubject`, `withOnTargetRemoved`, `getRelationTargets`, `Wildcard`, `Pair`, `isRelation`, `isWildcard` | Configured pair factories and inspection |
| Hierarchies | `Hierarchy`, `Cascade`, `getHierarchyDepth`, `getMaxHierarchyDepth` | Ordered/depth traversal; Cascade is an alias |
| Prefabs | `Prefab`, `addPrefab`, `IsA` | Template tag, creation, and inheritance relation |
| Convenience | `pipe`, `soa`, `aos` | Return-value composition and storage helpers |
| Internal access | `$internal` | Publicly exported escape hatch; avoid application dependence on private layout |

Public core type exports include `World`, `InternalWorld`, `WorldContext`, `EntityId`, `ComponentRef`, `ComponentData`, `ObservableHookDef`, `ObservableHook`, `QueryResult`, `Query`, `QueryOperatorType`, `OpReturnType`, `QueryOperator`, `QueryTerm`, `QueryOptions`, `HierarchyTerm`, `QueryModifier`, `OnTargetRemovedCallback`, `Relation`, and `RelationTarget`. Some useful types such as `EntityIndex` are not re-exported by the core index; infer them from factories rather than importing unsupported internal package paths. Core `ComponentRef` is permissive (`any`), so feature APIs provide stronger invariants than the library alone. [Exported types](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/index.ts).

### Serialization runtime exports: `bitecs/serialization`

| Category | Public values |
| --- | --- |
| Raw SoA | `createSoASerializer`, `createSoADeserializer` |
| Raw AoS | `createAoSSerializer`, `createAoSDeserializer` |
| Structural stream | `createObserverSerializer`, `createObserverDeserializer` |
| Snapshot | `createSnapshotSerializer`, `createSnapshotDeserializer` |
| Numeric tags | `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`, `f64` |
| Other tags | `str`, `array`, `ref` |
| Tag symbols | `$u8`, `$i8`, `$u16`, `$i16`, `$u32`, `$i32`, `$f32`, `$f64`, `$str`, `$ref` |

Public serialization type exports are `PrimitiveBrand`, `AoSSerializerOptions`, `AoSDeserializerOptions`, `ObserverSerializerOptions`, and `ObserverDeserializerOptions`. Prefer the functions over manipulating tag symbols directly. SoA option types are present in their implementation module but not re-exported by the entry point. [Serialization exports](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization/index.ts).

The package manifest also exposes a compatibility subpath, and the repository contains other source utilities. Neither is evidence that those utilities are part of the current supported modern public entry points. Use only exported entry points unless deliberately accepting maintenance of a fork. [Package exports](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/package.json).

## Documentation discrepancies and production boundaries

A current-version site can still contain inaccurate examples. The following source-backed findings should shape adoption and tests:

| Topic | Current finding | Practical consequence |
| --- | --- | --- |
| Reset | Source replaces registrations, observers attached there, query caches, and entity index | Rebuild setup; do not assume only entities were cleared |
| User data cleanup | Entity/component removal updates membership but does not erase your stores | Initialize reused slots and release owned object/resource references |
| Plain `getComponent` | Access is mediated by `onGet` callbacks; no automatic SoA row extraction | Install the contract or read owned fields directly |
| Setter initialization | `onAdd` fires before the initial setter payload | Do not read initialized payload from `onAdd` without a protocol |
| Query logic | Compilation supports simple mask operators, with flattened `Or` handling | Test boolean logic; avoid arbitrary nested/multiple independent groups |
| Hierarchy depth | Numeric depth is exact selection; other terms are not applied on that path | Filter explicitly or use a verified wrapper |
| Prefab values | Inheritance copies through getter/setter hooks at attachment time | Do not assume live propagation from a template |
| Target-removed callback | Type declares two parameters; runtime supplies world plus two IDs | Isolate and verify the adapter before use |
| Buffered queries | May fall back to ordinary ArrayBuffer; views/allocations can change | Check shared-memory capability and refresh current work views |
| Query teardown | `removeQuery` removes top-level entries but does not comprehensively unlink all references | Do not rely on it for unbounded dynamic-query lifecycle cleanup |
| Snapshot restore | Adds/upserts through an ID map, rather than deleting unrelated prior state | Prefer a fresh destination world for replacement loads |
| Snapshot coverage | Captures selected schemas, not all context/external resources/arbitrary AoS | Define an application save contract |
| Observer encoding | Subscribes to future events; relation tracking is not a general graph-change log | Explicit bootstrap; verify each relation mutation pattern |
| Serializer memory | Encoder defaults allocate 100 MiB backing buffers | Supply measured buffers and handle capacity policy |
| Wire schema | Component/property indices and limited-width counters/masks are encoded | Version the protocol; test schema and scale boundaries |
| License metadata | Homepage footer says MIT, but current package manifest says MPL-2.0 | Use the published package/license evidence when evaluating adoption |

Sources: [world lifecycle](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/World.ts), [entity lifecycle](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Entity.ts), [component/inheritance implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Component.ts), [query implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Query.ts), [hierarchy implementation](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/core/Hierarchy.ts), [serializer source directory](https://github.com/NateTheGreatt/bitECS/tree/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/src/serialization), [package manifest](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/package.json), [homepage](https://bitecs.dev/).

These findings do not require abandoning the library. They support a deliberately small adoption boundary: depend directly on straightforward membership/query/store operations, then add relationships, hierarchy, prefabs, or replication where their measured benefit exceeds the verification and ownership cost.

### Verification performed for this research

Release provenance and behavior were independently checked against a temporary installation of published `bitecs@0.4.0`, under Bun 1.4.3. The checks covered unhandled setters/getters, observer order, prefab value copying, retained storage and immediate reuse, reset behavior, multiple `Or` groups, hierarchy filtering/reparenting, buffered-query representation, recycled-ID diff state, nested-float-array diffs, snapshot merge behavior, and separate-world serializer roundtrips with ID remapping. The published core and serialization bundles matched the pinned source revision. These are focused compatibility probes, not a benchmark or an assertion that every API combination is correct.

All 12 complete example/store-allocation blocks passed execution under Bun and strict TypeScript 7.0.2 checking against the published declarations. The query, system-order, lifecycle, observer-queue, and delta continuations also passed when combined with their stated context and consolidated imports. The browser-loop sketch was typechecked with its context; it was not exercised in a browser. The nested sensing sketch uses placeholder application components and is illustrative. Worker synchronization, browser isolation headers, and real network transport were not integration-tested.

The cited issues #209, #213, and #214 were open when checked on October 18, 2026; their failure patterns above were reproduced against the released package rather than inferred solely from issue descriptions. Reports about a particular runtime should not be generalized to all Bun or browser use without reproducing the relevant workload.

## Official documentation and example inventory

This guide includes representative original snippets across the API surface. The links below preserve access to the complete official examples without duplicating the site's entire code collection. Read them as teaching material, and apply the 0.4.0 source caveats above before production use.

### Documentation snippets

| Official page | Snippet topics available |
| --- | --- |
| [Introduction](https://bitecs.dev/docs/introduction) | Complete entity/component/query movement walkthrough |
| [Installation](https://bitecs.dev/docs/installation) | Package setup, TypeScript configuration, import styles |
| [Quick Start](https://bitecs.dev/docs/quick-start) | World/stores/spawning, query/update, Canvas rendering, full movement demo |
| [World](https://bitecs.dev/docs/world) | Basic world, resources/context, separate worlds, shared index, versioning, lifecycle/inspection |
| [Entity](https://bitecs.dev/docs/entity) | Creation/removal, inspection, recycling, deferred query removals, versioned handles |
| [Component](https://bitecs.dev/docs/component) | SoA, typed/shared arrays, AoS, tags, batch membership, setter hooks, multiple-world storage |
| [Query](https://bitecs.dev/docs/query) | Basic/boolean selection, deferred removals, nested calls, buffered/options forms, hierarchy examples |
| [System](https://bitecs.dev/docs/system) | Function systems, ordering/pipelines, conditional cadence, delta/fixed time, browser/headless loops |
| [Relationships](https://bitecs.dev/docs/relationships) | Relation creation/data, exclusive and lifetime rules, target-removal callback, wildcard traversal, hierarchy |
| [Observers](https://bitecs.dev/docs/observers) | Membership transitions, getters/setters, validation, operators, queues, integration/disposal, derived data |
| [Prefabs](https://bitecs.dev/docs/prefabs) | Template creation/inheritance, composition and getters/setters |
| [Serialization](https://bitecs.dev/docs/serialization) | SoA/AoS, tags, strings/arrays, diffs/options, lifecycle replication, snapshots, ID mapping, interpolation |
| [Multithreading](https://bitecs.dev/docs/multithreading) | Shared components, buffered queries, worker jobs/queues, partitioning, physics/AI examples |
| [API reference](https://bitecs.dev/api) | Signatures and grouped function descriptions; less complete than the actual export inventory above |

The site's migration page is intentionally outside this research scope. Use the modern entry-point inventory and 0.4.0 examples above rather than importing an older schema/system/query paradigm.

### Interactive feature examples

The [official feature gallery](https://bitecs.dev/examples/) includes relationship following, prefab templates, lifecycle observers, boolean query selection, wildcard relationships, and exclusive targeting. Individual sections are reachable at [relationships](https://bitecs.dev/examples/#relationships), [prefabs](https://bitecs.dev/examples/#prefabs), [observers](https://bitecs.dev/examples/#observers), [query operators](https://bitecs.dev/examples/#query-operators), [wildcard queries](https://bitecs.dev/examples/#wildcards), and [exclusive relations](https://bitecs.dev/examples/#exclusive). The prefab propagation claim needs the copy-at-attachment caveat described above.

### Game-pattern examples

The [official pattern gallery](https://bitecs.dev/examples/patterns) includes all of these examples:

- [Particles](https://bitecs.dev/examples/patterns#particles)
- [Gravity](https://bitecs.dev/examples/patterns#gravity)
- [Top-down movement](https://bitecs.dev/examples/patterns#topdown)
- [Steering behaviors](https://bitecs.dev/examples/patterns#steering)
- [Collision detection](https://bitecs.dev/examples/patterns#collision)
- [Spawner pattern](https://bitecs.dev/examples/patterns#spawner)
- [Health and damage](https://bitecs.dev/examples/patterns#health)
- [State machine](https://bitecs.dev/examples/patterns#state)
- [Flocking](https://bitecs.dev/examples/patterns#flocking)

Treat these as small mechanic demonstrations. When adapting them, use seconds/ticks consistently, initialize every field a query reads, guard zero-distance normalization, and replace all-pairs neighborhood work when scale demands it. Physical gravity normally represents acceleration independent of mass; an example that multiplies gravity by mass is a gameplay model rather than a general physics rule. A “queued” particle effect function must also avoid hidden structural mutations inside observers if the goal is to defer them. These are review observations, not extra bitECS features.

The [demos page](https://bitecs.dev/examples/demos) links one integrated [Simple Game](https://bitecs.dev/examples/game), described as combining player movement, enemies, projectiles, collision, and a game loop. The game page was reachable, but its interactive behavior was not tested during this research. The repository also includes a first-party [networking example](https://github.com/NateTheGreatt/bitECS/tree/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/examples/networking), plus the source-backed [intro guide](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/docs/Intro.md), [serialization guide](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/docs/Serialization.md), and [multithreading guide](https://github.com/NateTheGreatt/bitECS/blob/1585cdb4ae9a6586aef5e4d9d30d7039e22cd543/docs/Multithreading.md). Prefer the package/source verification boundaries in this report whenever a guide and installed behavior disagree.
