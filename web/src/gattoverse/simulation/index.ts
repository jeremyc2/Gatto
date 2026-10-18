import { entityExists, hasComponent, query } from "bitecs";
import { get } from "./components";
import type { Command, GameView, RenderEntity, Simulation } from "./contracts";
import { ensureFlightRoofs, populateRoof } from "./generation";
import { findPath } from "./navigation";
import { behavior } from "./systems/behavior";
import { interactions } from "./systems/interactions";
import { integrate, playerIntent } from "./systems/movement";
import { survival } from "./systems/survival";
import { createGameWorld, isCurrent, kindOf, pointOf, stopInput, type GameWorld } from "./world";

const FIXED_STEP = 1 / 60;

function initialize(seed: number) {
  const world = createGameWorld(seed >>> 0);
  const roof = populateRoof(world, 0);
  world.components.Position.x[world.player] = roof.spawn.x;
  world.components.Position.y[world.player] = roof.spawn.y;
  return world;
}

function takeoff(world: GameWorld) {
  const { Inventory, Position, Velocity, Resident } = world.components;
  if (world.state.mode !== "roof" || !get(Inventory.jetpack, world.player)) return;
  ensureFlightRoofs(world, world.state.roofId + 4);
  const roof = world.state.platforms.get(world.state.roofId);
  if (!roof) return;
  const p = world.player;
  Position.x[p] = roof.x + roof.width - 45;
  Position.y[p] = roof.y - 135;
  Position.previousX[p] = get(Position.x, p);
  Position.previousY[p] = get(Position.y, p);
  Velocity.x[p] = 205;
  Velocity.y[p] = -80;
  Resident.roof[p] = -1;
  world.state.mode = "flight";
  world.state.phase = "playing";
  world.state.grace = 0.7;
  world.state.revision++;
  stopInput(world);
}

function renderEntity(world: GameWorld, id: number): RenderEntity {
  const { Visual, Loot, Brain } = world.components;
  return {
    id,
    ...pointOf(world, id),
    kind: kindOf(world, id),
    direction: get(Visual.direction, id),
    gait: get(Visual.gait, id),
    variant: get(Visual.variant, id),
    scale: get(Visual.scale, id),
    used: hasComponent(world, id, Loot) && get(Loot.used, id) === 1,
    state: hasComponent(world, id, Brain) ? get(Brain.state, id) : 0,
  };
}

/** No DOM, Phaser, storage, timers, or global random state lives behind this interface. */
export function createSimulation(seed: number): Simulation {
  let world = initialize(seed),
    accumulator = 0;
  function send(command: Command) {
    if (command.type === "restart") {
      world = initialize(command.seed);
      accumulator = 0;
      return;
    }
    if (command.type === "resume") {
      if (world.state.phase !== "over") {
        world.state.phase = "playing";
        world.state.dialogue = undefined;
        stopInput(world);
      }
      return;
    }
    if (command.type === "help" || command.type === "pause") {
      if (world.state.phase === "playing") {
        world.state.phase = command.type === "help" ? "help" : "paused";
        stopInput(world);
      }
      return;
    }
    if (command.type === "launch") {
      if (world.state.phase === "playing" || world.state.phase === "paused") takeoff(world);
      return;
    }
    if (world.state.phase !== "playing") return;
    if (command.type === "move")
      world.state.move = {
        x: Math.max(-1, Math.min(1, command.x)),
        y: Math.max(-1, Math.min(1, command.y)),
      };
    else if (command.type === "thrust") world.state.thrust = command.active;
    else if (command.type === "interact") {
      world.state.interact = true;
      world.state.interactTarget = undefined;
    } else if (command.type === "walk-to" && world.state.mode === "roof") {
      const roof = world.state.roofs.get(world.state.roofId);
      if (!roof) return;
      if (
        command.entity !== undefined &&
        (!entityExists(world, command.entity) || !isCurrent(world, command.entity))
      )
        return;
      world.state.target = {
        x: Math.max(35, Math.min(roof.width - 35, command.point.x)),
        y: Math.max(35, Math.min(roof.depth - 35, command.point.y)),
        ...(command.entity === undefined ? {} : { entity: command.entity }),
        path: findPath(world, pointOf(world, world.player), command.point),
      };
    }
  }

  function step(seconds: number) {
    if (world.state.phase !== "playing") {
      accumulator = 0;
      return;
    }
    accumulator += Math.max(0, Math.min(seconds, 0.25));
    while (accumulator + 1e-9 >= FIXED_STEP && world.state.phase === "playing") {
      accumulator -= FIXED_STEP;
      playerIntent(world, FIXED_STEP);
      behavior(world, FIXED_STEP);
      integrate(world, FIXED_STEP);
      interactions(world);
      if (world.state.phase === "playing") survival(world, FIXED_STEP);
      world.state.time += FIXED_STEP;
    }
  }

  function read(): GameView {
    const { Inventory, Visual } = world.components,
      p = world.player;
    const entities = [];
    for (const e of query(world, [Visual])) {
      if (!isCurrent(world, e)) continue;
      const entity = renderEntity(world, e);
      if (entity.used && entity.kind !== "case") continue;
      entities.push(entity);
    }
    return {
      seed: world.seed,
      phase: world.state.phase,
      mode: world.state.mode,
      time: world.state.time,
      revision: world.state.revision,
      roof: populateRoof(world, world.state.roofId),
      platforms: [...world.state.platforms.values()],
      player: renderEntity(world, p),
      entities,
      stamina: get(Inventory.stamina, p),
      keys: get(Inventory.keys, p),
      wallet: get(Inventory.wallet, p),
      score: get(Inventory.score, p),
      clothes: get(Inventory.clothes, p),
      jetpack: get(Inventory.jetpack, p) === 1,
      thrust: world.state.thrust || world.state.move.y < 0 || world.state.grace > 0,
      near: world.state.near,
      feedback: world.state.feedback,
      dialogue: world.state.dialogue,
      failure: world.state.failure,
    };
  }
  return { send, step, read };
}
