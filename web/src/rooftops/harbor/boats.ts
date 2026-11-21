import { clamp, remEuclid, type ArtPath, type Point2, type Rect4 } from "../types";
import { tugChannel } from "./bridge";
import {
  FERRY_DEPTH,
  FOREGROUND_BOAT_SPEED,
  LIGHT,
  MOON_CENTER,
  pushOval,
  pushQuad,
  pushShape,
  pushStroke,
  SAILBOAT_DEPTH,
  TRIM,
  TUG_DEPTH,
  TUG_PHASE,
  TUG_TURN_DISTANCE,
  View,
  WAKE_STREAKS,
} from "./projection";

export function waterDetail(
  view: View,
  h: number,
  time: number,
  rect: (bounds: Rect4, ink: number, alpha: number) => void,
): void {
  const { width: w, horizon } = view;
  const depth = h - horizon;
  for (let i = 0; i < 8; i++) {
    rect([0, horizon + (depth * i) / 8, w, depth / 8 + 0.2], 0x080d20, i * 0.058);
  }
  for (let i = 0; i < 68; i++) {
    const d = ((i * 47 + 2) % 101) / 101;
    const y = horizon + 3 + (depth - 5) * d;
    const base = ((i * 197 + 57) % 997) / 997;
    const rawX = remEuclid(base * w + time * (0.35 + d * 1.28), w + 56) - 28;
    const [[x, by], zoom] = view.billboard(rawX, -120 * d, y);
    const length = (7 + (i % 11) * 4) * (0.4 + d) * zoom;
    const glint = Math.sin(time * (0.41 + (i % 5) * 0.048) + i * 1.61);
    rect([x, by, length, 0.65 + d * 0.8], 0x66749c, 0.1 + (glint + 1) * 0.095);

    const reflection = MOON_CENTER[0] + Math.sin(i * 2.39 + time * 0.39) * (3 + d * 34);
    if (i % 2 === 0) {
      rect(
        [reflection - length * 0.24, by, length * 0.48, 0.8],
        0xc1a2a1,
        0.06 + (glint + 1) * 0.095,
      );
    }
  }
}

function tugRoute([start, end]: Point2, time: number): [number, number] {
  const distance = end - start;
  const leg = distance + 2 * TUG_TURN_DISTANCE;
  const trip = remEuclid(time * (FOREGROUND_BOAT_SPEED / 3) + leg * TUG_PHASE, 2 * leg);
  const [along, facing] = trip < leg ? [trip, 1] : [2 * leg - trip, -1];
  const easing = (d: number) => (d * d) / (4 * TUG_TURN_DISTANCE);
  const position =
    along < 2 * TUG_TURN_DISTANCE
      ? easing(along)
      : leg - along < 2 * TUG_TURN_DISTANCE
        ? distance - easing(leg - along)
        : along - TUG_TURN_DISTANCE;
  return [start + position, facing];
}

function foregroundPassage(view: View, time: number, phase: number): number {
  const length = view.width + 470;
  return -185 + length * remEuclid((time * FOREGROUND_BOAT_SPEED) / length + phase, 1);
}

function positionBoat(paths: ArtPath[], view: View, [x, depth]: Point2, size: number): ArtPath[] {
  const [left, waterline] = view.onWater(x, depth);
  const zoom = view.magnification(x, depth) * size;
  for (const path of paths) {
    for (const pt of path.points) {
      pt[0] = left + (pt[0] - x) * zoom;
      pt[1] = waterline + pt[1] * zoom;
    }
  }
  return paths;
}

function deckRail(paths: ArtPath[], [x, y]: Point2, length: number): void {
  pushStroke(paths, [x, y - 2.9], [x + length, y - 2.9], 0.65, 0x96899c, 0.86);
  for (const amount of [0.04, 0.5, 0.96]) {
    const px = x + length * amount;
    pushStroke(paths, [px, y - 2.9], [px, y], 0.6, 0x96899c, 0.8);
  }
}

function wake(
  paths: ArtPath[],
  [sternX, sternY]: Point2,
  facing: number,
  time: number,
  size: number,
): void {
  for (let i = 0; i < WAKE_STREAKS; i++) {
    const age = i;
    const x = sternX + facing * (1 - age * 10) * size;
    const offset = Math.sin(time * 1.35 + age * 0.66) * (age * 0.35) * size;
    const y = sternY + age * 0.9 * size;
    pushStroke(
      paths,
      [x, y + offset],
      [x - facing * (9 + age) * size, y + offset * 0.75],
      size,
      0x8994aa,
      0.36 * (1 - age / WAKE_STREAKS),
    );
  }
}

export function ferry(view: View, time: number): ArtPath[] {
  const x = foregroundPassage(view, time, 0.32);
  const facing = 1;
  const y = Math.sin(time * 1.45) * 0.9;
  const hull: Point2[] = [
    [x - 26 * facing, y - 7],
    [x + 31 * facing, y - 7],
    [x + 23 * facing, y],
    [x - 21 * facing, y],
  ];
  const paths: ArtPath[] = [];
  wake(paths, hull[3]!, facing, time, 1);
  pushShape(paths, hull, 0x080e1b, 1);
  pushQuad(paths, [x - 23, y - 7.5, 46, 1.5], 0x9a6b70, 0.82);
  pushStroke(paths, [x - 21, y - 3.8], [x + 23, y - 3.8], 0.65, 0x616578, 0.77);
  deckRail(paths, [x - 24, y - 7.5], 8);
  deckRail(paths, [x + 17, y - 7.5], 11);
  pushQuad(paths, [x - 16, y - 16, 33, 8], 0x3e4154, 1);
  pushQuad(paths, [x - 15, y - 10, 31, 1.1], 0x64596b, 1);
  pushQuad(paths, [x - 19, y - 17, 39, 1.7], TRIM, 1);
  pushQuad(paths, [x - 4, y - 23, 8, 6], 0x64596b, 1);
  pushQuad(paths, [x - 2.9, y - 21.7, 5.8, 2.25], 0x9aabc2, 0.88);
  pushQuad(paths, [x - 4.7, y - 23.4, 9.4, 0.9], TRIM, 1);
  pushQuad(paths, [x - 0.5, y - 29, 1, 7], TRIM, 1);
  pushQuad(paths, [x - 1, y - 29, 2, 2], LIGHT, 0.82);
  pushQuad(paths, [x + 24, y - 9, 1.65, 1.3], 0x9dbead, 0.86);
  for (let i = 0; i < 5; i++) {
    pushQuad(paths, [x - 13 + i * 6, y - 14, 3, 3], LIGHT, 0.84);
  }
  return positionBoat(paths, view, [x, FERRY_DEPTH], 0.9);
}

export function tugboat(view: View, time: number): ArtPath[] {
  const [start, end] = tugChannel(view);
  const [x, facing] = tugRoute([start, end], time);
  const y = Math.sin(time * 1.2 + 0.8) * 0.65;
  const at = (pts: readonly Point2[]): Point2[] => pts.map(([px, py]) => [x + px * facing, y + py]);
  const paths: ArtPath[] = [];
  const behind = facing > 0 ? x - start : end - x;
  const wakeSize = clamp(behind / 104, 0.13, 0.56);
  wake(paths, [x - facing * 13, y], facing, time, wakeSize);
  pushShape(
    paths,
    at([
      [-17, -5],
      [17, -5],
      [13, 0],
      [-13, 0],
    ]),
    0x101422,
    1,
  );
  pushQuad(paths, [x - 15, y - 5.5, 30, 1.5], 0x965d61, 0.86);
  for (const along of [-12, 11]) {
    const fender = x + along * facing;
    pushOval(paths, [fender, y - 3.4], [1.5, 1.7], 0x1d1b2a, 1);
    pushOval(paths, [fender, y - 3.4], [0.64, 0.77], 0x5f5263, 0.9);
  }
  deckRail(paths, [x + facing * 9, y - 5.4], facing * 5.4);
  pushShape(
    paths,
    at([
      [-9, -5],
      [-9, -12],
      [1, -12],
      [7, -5],
    ]),
    0x7e6874,
    1,
  );
  pushQuad(paths, [x - 5, y - 8.8, 10, 1.05], 0x9a7c81, 1);
  pushQuad(paths, [x - 4, y - 16, 11, 5], 0x9c8584, 1);
  pushQuad(paths, [x - 5, y - 17.5, 13, 1.8], 0x332c43, 1);
  pushQuad(paths, [x - 2.5, y - 15, 3, 2.5], LIGHT, 0.74);
  pushQuad(paths, [x + 2, y - 15, 3, 2.5], LIGHT, 0.74);
  const stack = x - facing * 8;
  pushQuad(paths, [stack - 2, y - 21, 4, 10], 0x615263, 1);
  pushQuad(paths, [stack - 1.55, y - 19, 0.9, 7.2], 0x827081, 1);
  pushQuad(paths, [stack - 2.8, y - 21, 5.6, 2], 0x9d7576, 1);
  pushOval(paths, [stack, y - 21], [2.6, 0.8], 0x393043, 1);
  for (let puff = 0; puff < 5; puff++) {
    const age = remEuclid(time * 0.47 + puff / 5, 1);
    const drift = Math.sin(time * 1.7 + puff * 2.4) * age * 1.2;
    const cx = stack - facing * age * 13 + drift;
    const cy = y - 21.4 - age * 15;
    const r = 1.3 + age * 3;
    const opacity = Math.min(age * 14, 1) * (1 - age) * (1 - age) * 0.14;
    pushOval(paths, [cx, cy], [r, r * 0.73], 0xb4adc2, opacity);
    pushOval(
      paths,
      [cx + drift * 0.65, cy - r * 0.38],
      [r * 0.74, r * 0.55],
      0xa39bb7,
      opacity * 0.6,
    );
  }
  return positionBoat(paths, view, [x, TUG_DEPTH], 1);
}

export function sailboat(view: View, time: number): ArtPath[] {
  const x = foregroundPassage(view, time, 0.7);
  const facing = 1;
  const y = Math.sin(time * 1.3 + 1) * 1.15;
  const lean = Math.sin(time * 0.7) * 1.2;
  const at = (pts: readonly Point2[]): Point2[] => pts.map(([px, py]) => [x + px * facing, y + py]);
  const paths: ArtPath[] = [];
  wake(paths, [x - facing * 15, y], facing, time, 0.44);
  pushShape(
    paths,
    at([
      [-18, -4.7],
      [21, -4.7],
      [15, 0],
      [-15, 0],
    ]),
    0x2c2437,
    1,
  );
  pushQuad(paths, [x - 15, y - 5.3, 30, 1.3], 0xbb8c82, 0.77);
  pushStroke(paths, [x - 14, y - 2.5], [x + 16, y - 2.5], 0.65, 0x8b6b7b, 0.9);
  pushQuad(paths, [x - 5, y - 7, 10, 2], 0x68505e, 1);
  pushShape(
    paths,
    at([
      [lean - 1.5, -34],
      [-2, -8],
      [-17, -8],
      [-11, -17],
    ]),
    0xc7b0a4,
    0.93,
  );
  pushShape(
    paths,
    at([
      [lean + 1.5, -28],
      [3, -8.5],
      [16, -8.5],
      [11, -16],
    ]),
    0x817788,
    1,
  );
  pushStroke(paths, [x - 13.1, y - 14.5], [x - 1.5, y - 14.5], 0.62, 0x937d83, 0.65);
  pushStroke(paths, [x + 5, y - 13.3], [x + 12.4, y - 13.3], 0.55, 0xb2a0a6, 0.64);
  pushStroke(paths, [x - 16, y - 5.6], [x + lean - 0.6, y - 34.3], 0.46, 0x918799, 0.72);
  pushStroke(paths, [x, y - 5], [x + lean * facing, y - 35], 1.15, 0xb5a4aa, 1);
  pushQuad(paths, [x - facing * 11 - 1, y - 7, 2, 2], LIGHT, 0.88);
  return positionBoat(paths, view, [x, SAILBOAT_DEPTH], 0.82);
}
