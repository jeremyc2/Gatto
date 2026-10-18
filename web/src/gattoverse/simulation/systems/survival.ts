import { query } from "bitecs";
import { get } from "../components";
import { cleanFlight, ensureFlightRoofs, populateRoof } from "../generation";
import { clamp, distance } from "../random";
import {
  evictOldRoofs,
  fail,
  isCurrent,
  kindOf,
  pointOf,
  stopInput,
  type GameWorld,
} from "../world";
import { canStand } from "./movement";

export function survival(world: GameWorld, dt: number) {
  const { Position, Collider, Hazard, Inventory } = world.components;
  const p = world.player,
    here = pointOf(world, p);
  const walked = Math.hypot(
    here.x - get(Position.previousX, p),
    here.y - get(Position.previousY, p),
  );
  const drain =
    world.state.mode === "flight"
      ? dt * (world.state.thrust || world.state.move.y < 0 ? 4.5 : 3.6)
      : walked * 0.009;
  Inventory.stamina[p] = Math.max(0, get(Inventory.stamina, p) - drain);
  if (!get(Inventory.stamina, p)) {
    fail(world, "hunger");
    return;
  }
  for (const e of query(world, [Hazard, Position, Collider])) {
    if (isCurrent(world, e) && distance(here, pointOf(world, e)) < get(Collider.radius, e) + 12) {
      const kind = kindOf(world, e);
      fail(world, kind === "rat" || kind === "owl" || kind === "unicorn" ? kind : "fall");
      return;
    }
  }
  if (world.state.mode !== "flight") return;
  if (here.y > 810) {
    fail(world, "fall");
    return;
  }
  world.state.grace = Math.max(0, world.state.grace - dt);
  ensureFlightRoofs(
    world,
    world.state.nextPlatform + (world.state.flightEnd - here.x < 2300 ? 1 : -1),
  );
  cleanFlight(world, here.x - 2000);
  if (world.state.grace > 0) return;
  for (const platform of world.state.platforms.values()) {
    if (here.x < platform.x - 10 || here.x > platform.x + platform.width + 10) continue;
    if (get(Position.previousY, p) + 19 <= platform.y && here.y + 19 >= platform.y) {
      world.state.roofId = platform.id;
      const roof = populateRoof(world, platform.id);
      world.state.mode = "roof";
      let x = clamp((here.x - platform.x) / platform.width, 0.15, 0.85) * roof.width;
      let y = roof.depth * 0.68;
      if (!canStand(world, x, y, 24)) {
        x = roof.spawn.x;
        y = roof.spawn.y;
      }
      Position.x[p] = x;
      Position.y[p] = y;
      world.components.Resident.roof[p] = platform.id;
      world.state.revision++;
      stopInput(world);
      evictOldRoofs(world);
      break;
    }
    if (here.y > platform.y + 24) {
      fail(world, "fall");
      return;
    }
  }
}
