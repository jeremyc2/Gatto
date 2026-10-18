import type { Point } from "../simulation/contracts";
export const project = (p: Point): Point => ({ x: (p.x - p.y) * 0.78, y: (p.x + p.y) * 0.38 });
export const unproject = (p: Point): Point => ({
  x: (p.x / 0.78 + p.y / 0.38) / 2,
  y: (p.y / 0.38 - p.x / 0.78) / 2,
});
