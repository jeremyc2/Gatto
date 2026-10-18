import { hasComponent, query } from "bitecs";
import { get } from "../components";
import { clamp, distance } from "../random";
import { isCurrent, kindOf, pointOf, type GameWorld } from "../world";
import { canStand } from "./movement";

export function behavior(world: GameWorld, dt: number) {
  const { Brain, Position, Velocity, Visual, Visitor, Hazard } = world.components;
  const player = pointOf(world, world.player),
    roof = world.state.roofs.get(world.state.roofId);
  for (const e of query(world, [Brain, Position, Velocity])) {
    if (!isCurrent(world, e)) continue;
    const p = pointOf(world, e),
      kind = kindOf(world, e),
      range = distance(p, player);
    const isVisitor = hasComponent(world, e, Visitor),
      hostile = hasComponent(world, e, Hazard);
    let timer = get(Brain.timer, e) - dt,
      state = get(Brain.state, e);
    if (
      hostile &&
      kind !== "owl" &&
      (world.state.mode === "roof" || kind === "unicorn") &&
      state !== 2 &&
      state !== 3 &&
      range < (kind === "unicorn" ? 290 : 225)
    ) {
      state = 2;
      timer = kind === "unicorn" ? 4.2 : 2.8;
    }
    if (state === 2 && timer <= 0) {
      state = 3;
      timer = 3;
    }
    let tx = get(Brain.targetX, e),
      ty = get(Brain.targetY, e);
    if (state === 2) {
      tx = player.x;
      ty = player.y;
    } else if (state === 3) {
      tx = get(Brain.homeX, e);
      ty = get(Brain.homeY, e);
      if (distance(p, { x: tx, y: ty }) < 24) {
        state = 0;
        timer = 2.5;
      }
    } else if (kind === "owl") {
      tx = get(Brain.homeX, e) + Math.sin(world.state.time * 0.6 + e) * 130;
      ty = get(Brain.homeY, e) + Math.cos(world.state.time * 1.15 + e) * 65;
      state = 1;
    } else if (timer <= 0) {
      if (isVisitor && range < 220 && range > 90 && world.rng.next() < 0.38) {
        state = 4;
        timer = world.rng.between(1.2, 2.5);
      } else if (world.rng.next() < 0.28) {
        state = 0;
        timer = world.rng.between(1.2, 4.5);
      } else {
        state = 1;
        timer = world.rng.between(2.5, 6);
        for (let attempt = 0; attempt < 20; attempt++) {
          tx = get(Brain.homeX, e) + world.rng.between(-240, 240);
          ty = get(Brain.homeY, e) + world.rng.between(-200, 200);
          if (world.state.mode === "roof" && roof) {
            tx = clamp(tx, 60, roof.width - 60);
            ty = clamp(ty, 60, roof.depth - 60);
          } else if (kind === "rat") {
            tx = get(Brain.homeX, e) + world.rng.between(-65, 65);
            ty = get(Brain.homeY, e);
          } else if (kind === "unicorn") {
            tx = get(Brain.homeX, e) + world.rng.between(-180, 180);
            ty = clamp(ty, 100, 350);
          }
          if (world.state.mode === "flight" || canStand(world, tx, ty, 20)) break;
        }
      }
    }
    if (state === 4) {
      tx = player.x;
      ty = player.y;
      if (range < 85) state = 0;
    }
    // A visitor stops to acknowledge nearby paws, and turns its actual directional sprite.
    if (isVisitor && range < 95) {
      state = 0;
      const sx = (player.x - p.x - player.y + p.y) * 0.78,
        sy = (player.x - p.x + player.y - p.y) * 0.38;
      Visual.direction[e] = (Math.round(Math.atan2(sy, sx) / (Math.PI / 4)) + 8) % 8;
    }
    const dx = tx - p.x,
      dy = ty - p.y,
      length = Math.hypot(dx, dy);
    const speed = get(Brain.speed, e) * (state === 3 ? 0.65 : state === 2 ? 1 : 0.6);
    Velocity.x[e] = state && length > 10 ? (dx / length) * speed : 0;
    Velocity.y[e] = state && length > 10 ? (dy / length) * speed : 0;
    Brain.targetX[e] = tx;
    Brain.targetY[e] = ty;
    Brain.timer[e] = timer;
    Brain.state[e] = state;
  }
}
