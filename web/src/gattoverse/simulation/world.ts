import { addComponents, addEntity, createWorld, query, removeEntity } from "bitecs";
import { components, get, kindIndex, spriteKinds } from "./components";
import type {
  ActorKind,
  Failure,
  FlightRoof,
  GameView,
  Phase,
  Point,
  RoofLayout,
  SpriteKind,
} from "./contracts";
import { random, type Random } from "./random";

export function createGameWorld(seed: number) {
  const world = createWorld({
    components: components(),
    seed,
    rng: random(seed),
    player: 0,
    state: {
      mode: "roof" as "roof" | "flight",
      phase: "playing" as Phase,
      time: 0,
      revision: 0,
      roofId: 0,
      roofs: new Map<number, RoofLayout>(),
      platforms: new Map<number, FlightRoof>(),
      move: { x: 0, y: 0 },
      target: undefined as (Point & { entity?: number; path: Point[] }) | undefined,
      thrust: false,
      grace: 0,
      nextPlatform: 0,
      flightEnd: 0,
      interact: false,
      interactTarget: undefined as number | undefined,
      near: undefined as GameView["near"],
      feedback: undefined as GameView["feedback"],
      dialogue: undefined as GameView["dialogue"],
      failure: undefined as Failure,
    },
  });
  const p = spawn(world, "cat", { x: 0, y: 0 }, 0);
  world.player = p;
  addComponents(
    world,
    p,
    world.components.Player,
    world.components.Inventory,
    world.components.Collider,
  );
  world.components.Inventory.stamina[p] = 100;
  world.components.Collider.radius[p] = 14;
  return world;
}
export type GameWorld = ReturnType<typeof createGameWorld>;

export function spawn(
  world: { components: ReturnType<typeof components> } & ReturnType<typeof createWorld>,
  kind: SpriteKind,
  point: Point,
  roof: number,
  scale = 1,
  variant = 0,
) {
  const e = addEntity(world);
  if (e >= 32768) throw new Error("Gattoverse entity capacity exceeded.");
  const { Position, Velocity, Visual, Resident } = world.components;
  addComponents(world, e, Position, Velocity, Visual, Resident);
  Position.x[e] = point.x;
  Position.y[e] = point.y;
  Position.previousX[e] = point.x;
  Position.previousY[e] = point.y;
  Velocity.x[e] = 0;
  Velocity.y[e] = 0;
  Visual.direction[e] = 0;
  Visual.gait[e] = 0;
  Visual.kind[e] = kindIndex(kind);
  Visual.scale[e] = scale;
  Visual.variant[e] = variant;
  Resident.roof[e] = roof;
  return e;
}

export function kindOf(world: GameWorld, e: number): SpriteKind {
  return spriteKinds[get(world.components.Visual.kind, e)] ?? "cat";
}
export const pointOf = (world: GameWorld, e: number) => ({
  x: get(world.components.Position.x, e),
  y: get(world.components.Position.y, e),
});
export const isCurrent = (world: GameWorld, e: number) =>
  get(world.components.Resident.roof, e) ===
  (world.state.mode === "roof" ? world.state.roofId : -1);

export function addBrain(world: GameWorld, e: number, speed: number, rng: Random) {
  const { Brain } = world.components;
  addComponents(world, e, Brain);
  const p = pointOf(world, e);
  Brain.homeX[e] = p.x;
  Brain.homeY[e] = p.y;
  Brain.targetX[e] = p.x;
  Brain.targetY[e] = p.y;
  Brain.speed[e] = speed;
  Brain.timer[e] = rng.between(0.5, 4);
  Brain.state[e] = 0;
}

export function addHazard(
  world: GameWorld,
  kind: "rat" | "owl" | "unicorn",
  p: Point,
  roof: number,
  rng: Random,
) {
  const e = spawn(world, kind, p, roof, kind === "unicorn" ? 1.6 : 1);
  addComponents(world, e, world.components.Hazard, world.components.Collider);
  world.components.Collider.radius[e] = kind === "unicorn" ? 26 : kind === "owl" ? 18 : 14;
  addBrain(world, e, kind === "unicorn" ? 228 : kind === "owl" ? 70 : 112, rng);
  return e;
}

export function addVisitor(world: GameWorld, kind: ActorKind, p: Point, roof: number, rng: Random) {
  const e = spawn(world, kind, p, roof, kind === "velvet" ? 1.55 : kind === "paper" ? 1.15 : 1);
  addComponents(world, e, world.components.Visitor);
  world.components.Visitor.conversations[e] = 0;
  addBrain(world, e, rng.between(25, 48), rng);
  return e;
}

export function feedback(
  world: GameWorld,
  icon: NonNullable<GameView["feedback"]>["icon"],
  amount = 1,
) {
  world.state.feedback = { icon, amount, serial: (world.state.feedback?.serial ?? 0) + 1 };
}

export function stopInput(world: GameWorld) {
  world.state.move = { x: 0, y: 0 };
  world.state.target = undefined;
  world.state.thrust = false;
  world.state.interact = false;
  world.state.interactTarget = undefined;
}
export function fail(world: GameWorld, cause: Failure) {
  if (world.state.phase !== "playing") return;
  world.state.phase = "over";
  world.state.failure = cause;
  stopInput(world);
}

export function evictOldRoofs(world: GameWorld) {
  const { Resident } = world.components;
  for (const e of query(world, [Resident])) {
    const roof = get(Resident.roof, e);
    if (e !== world.player && roof >= 0 && roof < world.state.roofId - 3) removeEntity(world, e);
  }
  for (const id of world.state.roofs.keys())
    if (id < world.state.roofId - 3) world.state.roofs.delete(id);
}
