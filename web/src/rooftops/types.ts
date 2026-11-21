export const STEP = 1 / 120;
export const SPEED = 310;
export const GRAVITY = 1450;
export const JUMP = 535;
export const LEVELS = 3;

export const SKY = 0x101322;
export const COURSE_MOON_DIAMETER = 94;
export const GOLD = 0xffdba0;
export const PURPLE = 0x8b7bff;
export const START_X = 105;
export const DESTINATION_INSET = 140;
export const REST_ROOF_WIDTH = 400;
// A 27-unit cat body is about 0.42 m; most rooftop gaps are about 2 m.
export const WORLD_UNITS_PER_METER = 64;
export const PLAYER_AHEAD_GREEN = 0x3ddc97;
export const PLAYER_BEHIND_RED = 0xff6b81;

export const SCORE_BUTTON_NORMAL = 0xffdfac;
export const SCORE_BUTTON_HOVER = 0xffedc9;
export const SCORE_BUTTON_ACTIVE = 0xe3b66e;

export const ARRIVAL_DURATION = 2.6;
export const ARRIVAL_FLAME_OUT = 1.0;
export const ARRIVAL_LANDING = 1.7;
export const ARRIVAL_UNSTRAP = 1.9;

export const POSTER_FONT =
  '"Helvetica Neue", -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif';

export type Point2 = [number, number];
export type Point3 = [number, number, number];
export type Rect4 = [number, number, number, number];

export interface Roof {
  readonly x: number;
  readonly y: number;
  readonly width: number;
}

export interface Coin {
  x: number;
  y: number;
  collected: boolean;
}

export type EnemyKind = "rat" | "owl" | "bat";

export interface Enemy {
  roof: number;
  kind: EnemyKind;
  defeated: boolean;
  phase: number;
  speed: number;
}

export interface Input {
  left: boolean;
  right: boolean;
  jump: boolean;
}

export interface Pose {
  x: number;
  y: number;
  facing: number;
  gait: number;
  squash: number;
  home: boolean;
}

export interface Frame {
  seconds: number;
  level: number;
  returns: number;
  pose: Pose;
}

export interface Replay {
  seconds: number;
  frames: Frame[];
}

export interface Score {
  coins: number;
  deaths: number;
  kills: number;
  seconds: number;
  personalBest: number | null;
}

export interface Preferences {
  musicVolume: number;
  effectsVolume: number;
  ghost: boolean;
  animations: boolean;
}

export interface ArtPath {
  points: Point2[];
  ink: number;
  opacity: number;
}

export interface PosterShape {
  rect: Rect4;
  ink: number;
  opacity: number;
  radius: number;
}

export interface PosterText {
  label: string;
  x: number;
  y: number;
  size: number;
  ink: number;
  bold: boolean;
  centeredWidth: number | null;
}

export interface CheckpointPalette {
  wall: number;
  coping: number;
  frame: number;
}

export interface GhostGap {
  ahead: boolean;
  meters: number;
}

export type StatIcon = "coin" | "deaths" | "kills" | "time";

export type NeighborhoodId = "brownstone" | "loft" | "limestone";

const hexCache = new Map<number, string>();

/** Convert a 24-bit RGB integer to a cached `#rrggbb` string without per-frame allocations. */
export function rgbCss(ink: number): string {
  const key = ink & 0xffffff;
  let cached = hexCache.get(key);
  if (cached === undefined) {
    cached = `#${key.toString(16).padStart(6, "0")}`;
    hexCache.set(key, cached);
  }
  return cached;
}

export function remEuclid(value: number, divisor: number): number {
  const r = value % divisor;
  return r < 0 ? r + divisor : r;
}

export function clamp(value: number, min: number, max: number): number {
  return value < min ? min : value > max ? max : value;
}

export function timeLabel(seconds: number): string {
  const whole = Math.floor(Math.max(0, seconds));
  const mins = Math.floor(whole / 60);
  const secs = whole % 60;
  if (whole >= 3600) {
    const hours = Math.floor(whole / 3600);
    return `${hours}:${String(mins % 60).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }
  return `${mins}:${String(secs).padStart(2, "0")}`;
}
