import { query } from "bitecs";
import { get } from "../components";
import { clamp, distance } from "../random";
import { isCurrent, kindOf, pointOf, type GameWorld } from "../world";
import { findPath } from "../navigation";

export function playerIntent(world: GameWorld, dt: number) {
  const { Velocity, Position } = world.components;
  const p = world.player,
    state = world.state;
  let dx = state.move.x,
    dy = state.move.y;
  if (state.mode === "flight") {
    Velocity.x[p] = 205 + dx * 45;
    const thrust = state.thrust || dy < -0.2 || state.grace > 0;
    Velocity.y[p] = clamp(
      get(Velocity.y, p) + (thrust ? -470 : dy > 0.2 ? 630 : 390) * dt,
      -235,
      300,
    );
    if (get(Position.y, p) < 42 && get(Velocity.y, p) < 0) Velocity.y[p] = 0;
    return;
  }
  if (dx || dy) state.target = undefined;
  if (state.target) {
    const target =
      state.target.entity === undefined ? state.target : pointOf(world, state.target.entity);
    const here = pointOf(world, p),
      remaining = distance(target, here);
    const kind = state.target.entity === undefined ? undefined : kindOf(world, state.target.entity);
    const reach = kind === "key" || kind === "coin" || kind === "food" ? 22 : kind ? 50 : 10;
    if (remaining < reach) {
      state.interact =
        kind === "case" ||
        kind === "vendor" ||
        kind === "watercolor" ||
        kind === "paper" ||
        kind === "velvet";
      state.interactTarget = state.target.entity;
      state.target = undefined;
    } else {
      const path = state.target.path;
      const last = path.at(-1);
      if (last && distance(last, target) > 100) state.target.path = findPath(world, here, target);
      if (path[0] && distance(here, path[0]) < 16) path.shift();
      const next = path[0];
      if (next) {
        const length = distance(next, here);
        dx = (next.x - here.x) / length;
        dy = (next.y - here.y) / length;
      } else {
        state.target = undefined;
        dx = 0;
        dy = 0;
      }
    }
  } else {
    // Browser controls use screen axes; the simulation converts them to the oblique roof plane.
    const screenX = dx,
      screenY = dy;
    dx = screenX / 0.78 + screenY / 0.38;
    dy = -screenX / 0.78 + screenY / 0.38;
  }
  const length = Math.hypot(dx, dy);
  Velocity.x[p] = length ? (dx / length) * 245 : 0;
  Velocity.y[p] = length ? (dy / length) * 245 : 0;
}

export function canStand(world: GameWorld, x: number, y: number, radius: number) {
  const { Obstacle, Resident, Position } = world.components;
  for (const e of query(world, [Obstacle, Resident, Position])) {
    if (!isCurrent(world, e)) continue;
    if (
      Math.abs(x - get(Position.x, e)) < get(Obstacle.width, e) / 2 + radius &&
      Math.abs(y - get(Position.y, e)) < get(Obstacle.depth, e) / 2 + radius
    )
      return false;
  }
  return true;
}

export function integrate(world: GameWorld, dt: number) {
  const { Position, Velocity, Visual, Resident } = world.components;
  const roof = world.state.roofs.get(world.state.roofId);
  for (const e of query(world, [Position, Velocity, Resident])) {
    if (!isCurrent(world, e)) continue;
    const x = get(Position.x, e),
      y = get(Position.y, e);
    Position.previousX[e] = x;
    Position.previousY[e] = y;
    const vx = get(Velocity.x, e),
      vy = get(Velocity.y, e);
    if (!vx && !vy) {
      Visual.gait[e] = 0;
      continue;
    }
    let nx = x + vx * dt,
      ny = y + vy * dt;
    if (world.state.mode === "roof" && roof) {
      nx = clamp(nx, 35, roof.width - 35);
      ny = clamp(ny, 35, roof.depth - 35);
      if (!canStand(world, nx, y, 14)) nx = x;
      if (!canStand(world, nx, ny, 14)) ny = y;
      if (e === world.player && world.state.target && Math.hypot(nx - x, ny - y) < 0.2) {
        // Slide around isolated furniture instead of sticking to its face on tap-to-walk.
        const side = Math.sign(vx * vy || 1);
        if (canStand(world, x + side * 90 * dt, y, 14)) nx = x + side * 90 * dt;
      }
    }
    Position.x[e] = nx;
    Position.y[e] = ny;
    const moved = Math.hypot(nx - x, ny - y);
    Visual.gait[e] = moved > 0.01 ? get(Visual.gait, e) + moved / 17 : 0;
    const sx = world.state.mode === "roof" ? (nx - x - (ny - y)) * 0.78 : vx;
    const sy = world.state.mode === "roof" ? (nx - x + ny - y) * 0.38 : vy * 0.2;
    if (moved > 0.01)
      Visual.direction[e] = (Math.round(Math.atan2(sy, sx) / (Math.PI / 4)) + 8) % 8;
  }
}
