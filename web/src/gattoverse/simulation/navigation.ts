import { query } from "bitecs";
import { get } from "./components";
import type { Point } from "./contracts";
import { distance } from "./random";
import { isCurrent, type GameWorld } from "./world";

/** An invisible navigation grid routes taps around irregularly scattered roof furniture. */
export function findPath(world: GameWorld, from: Point, destination: Point): Point[] {
  const roof = world.state.roofs.get(world.state.roofId);
  if (!roof) return [];
  const { Position, Obstacle } = world.components;
  const blocks = [...query(world, [Obstacle, Position])]
    .filter((e) => isCurrent(world, e))
    .map((e) => ({
      x: get(Position.x, e),
      y: get(Position.y, e),
      w: get(Obstacle.width, e) / 2 + 24,
      h: get(Obstacle.depth, e) / 2 + 24,
    }));
  const clear = (p: Point) =>
    p.x >= 34 &&
    p.y >= 34 &&
    p.x <= roof.width - 34 &&
    p.y <= roof.depth - 34 &&
    !blocks.some((b) => Math.abs(p.x - b.x) < b.w && Math.abs(p.y - b.y) < b.h);
  const straight = (a: Point, b: Point) => {
    const steps = Math.ceil(distance(a, b) / 18);
    for (let i = 1; i <= steps; i++)
      if (!clear({ x: a.x + ((b.x - a.x) * i) / steps, y: a.y + ((b.y - a.y) * i) / steps }))
        return false;
    return true;
  };
  if (straight(from, destination)) return [destination];
  const cell = 44,
    columns = Math.ceil(roof.width / cell),
    rows = Math.ceil(roof.depth / cell);
  const point = (id: number) => ({
    x: ((id % columns) + 0.5) * cell,
    y: (Math.floor(id / columns) + 0.5) * cell,
  });
  const idAt = (p: Point) => Math.floor(p.y / cell) * columns + Math.floor(p.x / cell);
  const walkable = new Uint8Array(columns * rows);
  for (let i = 0; i < walkable.length; i++) walkable[i] = clear(point(i)) ? 1 : 0;
  const start = idAt(from);
  let goal = idAt(destination);
  if (!walkable[goal]) {
    let nearest = Infinity;
    for (let i = 0; i < walkable.length; i++) {
      const range = distance(point(i), destination);
      if (walkable[i] && range < nearest && straight(point(i), destination)) {
        nearest = range;
        goal = i;
      }
    }
  }
  const costs = new Float64Array(walkable.length).fill(Infinity),
    parents = new Int32Array(walkable.length).fill(-1);
  const open = new Set([start]),
    closed = new Set<number>();
  costs[start] = 0;
  while (open.size) {
    let current = -1,
      best = Infinity;
    for (const id of open) {
      const rank = get(costs, id) + distance(point(id), point(goal));
      if (rank < best) {
        best = rank;
        current = id;
      }
    }
    if (current === goal) {
      const path: Point[] = [destination];
      for (let at = current; at !== start && at >= 0; at = get(parents, at))
        path.unshift(point(at));
      const smooth: Point[] = [];
      let anchor = from;
      while (path.length) {
        let next = path.length - 1;
        while (next > 0 && !straight(anchor, path[next] ?? destination)) next--;
        anchor = path[next] ?? destination;
        smooth.push(anchor);
        path.splice(0, next + 1);
      }
      return smooth;
    }
    open.delete(current);
    closed.add(current);
    for (let y = -1; y <= 1; y++)
      for (let x = -1; x <= 1; x++) {
        if (!x && !y) continue;
        const column = (current % columns) + x,
          row = Math.floor(current / columns) + y;
        const next = row * columns + column;
        if (
          column < 0 ||
          column >= columns ||
          row < 0 ||
          row >= rows ||
          !walkable[next] ||
          closed.has(next)
        )
          continue;
        if (x && y && (!walkable[current + x] || !walkable[current + y * columns])) continue;
        const cost = get(costs, current) + cell * (x && y ? Math.SQRT2 : 1);
        if (cost < get(costs, next)) {
          costs[next] = cost;
          parents[next] = current;
          open.add(next);
        }
      }
  }
  return [];
}
