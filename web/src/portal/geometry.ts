export const TAU = Math.PI * 2;

export function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

export function mix(from: number, to: number, progress: number): number {
  return from + (to - from) * progress;
}

export function progress(start: number, end: number, value: number): number {
  return clamp((value - start) / (end - start), 0, 1);
}

export function ease(start: number, end: number, value: number): number {
  const t = progress(start, end, value);
  return t * t * (3 - 2 * t);
}

export function randomFrom(seed: number): () => number {
  return () => {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}

export interface PortalFrame {
  width: number;
  height: number;
  x: number;
  y: number;
  radius: number;
  time: number;
}
