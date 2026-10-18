export type Random = {
  next(): number;
  between(min: number, max: number): number;
  integer(min: number, max: number): number;
};

export function random(seed: number): Random {
  let state = seed >>> 0;
  const next = () => {
    state += 0x6d2b79f5;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
  return {
    next,
    between: (min, max) => min + next() * (max - min),
    integer: (min, max) => Math.floor(min + next() * (max - min + 1)),
  };
}

export const roofSeed = (seed: number, id: number) => (seed ^ Math.imul(id + 1, 0x9e3779b1)) >>> 0;
export const clamp = (n: number, min: number, max: number) => Math.max(min, Math.min(max, n));
export const distance = (a: { x: number; y: number }, b: { x: number; y: number }) =>
  Math.hypot(a.x - b.x, a.y - b.y);
