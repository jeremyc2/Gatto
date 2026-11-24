import { drawArtPaths } from "../art";
import { createTileCanvas, paintTile, type CachedTile } from "../rooms";
import type { ArtPath, Point2, Rect4 } from "../types";
import { Camera, HARBOR_WIDTH, mix3, pushQuad, pushShape, View } from "./projection";
import { STREET_DEPTH, streetGrade } from "./shore";

const SHAFT_TOP = 245;
const FLOOR = 12;
const SHAFT_HEIGHT = FLOOR * 6;
const BASE_HEIGHT = 58;
const GROUND = SHAFT_TOP + SHAFT_HEIGHT + BASE_HEIGHT;
export const LANDMARKS_DEPTH = 835;
const SCALE = 0.68;
const STONE = 0x414358;
const STEEL = 0x42465f;
const GLASS = 0xe0bf92;

const EMPIRE_X = 0.128 * HARBOR_WIDTH;
const CHRYSLER_X = 0.877 * HARBOR_WIDTH;
const ARCHES: readonly [number, number, number, number][] = [
  [31, 213, 27, 0x87829c],
  [27, 200, 25, 0x9794ae],
  [22, 188, 24, 0xa6a4b7],
  [16, 176, 22, 0xacaabc],
  [10, 163, 19, 0xb8b2c0],
];

type Entrance = "limestone" | "steel";

interface Landmark {
  tile: CachedTile | null;
  front: ArtPath[];
  width: number;
}

function windowBays(width: number): number[] {
  const bays = Math.max(1, Math.floor((width - 5) / 10));
  const pitch = (width - 6) / bays;
  const out: number[] = [];
  for (let bay = 0; bay < bays; bay++) {
    out.push(1.4 + (bay + 0.5) * pitch);
  }
  return out;
}

function addFacade(
  paths: ArtPath[],
  [x, y, width, height]: Rect4,
  ink: number,
  seed: number,
): void {
  pushQuad(paths, [x, y, width, height], ink, 1);
  pushQuad(paths, [x + width * 0.73, y, width * 0.27, height], 0x181c31, 0.36);
  pushQuad(paths, [x, y, 2, height], 0xaaa3ae, 0.27);
  const floors = Math.floor(height / FLOOR);
  const bays = windowBays(width);
  for (let bay = 0; bay < bays.length; bay++) {
    const wx = x + bays[bay]!;
    pushQuad(paths, [wx - 1.2, y, 0.65, height], 0x9a91a3, 0.29);
    for (let floor = 0; floor < floors; floor++) {
      const wy = y + (floor + 0.5) * FLOOR - 2.7;
      const lit = (seed + bay * (5 + floor) + floor * 11) % 17 < 6;
      pushQuad(paths, [wx, wy, 3.2, 5.4], GLASS, lit ? 0.7 : 0.22);
    }
  }
}

function addFrontage(paths: ArtPath[], width: number, entrance: Entrance, shaft: number): void {
  const [stone, trim, metal, portal] =
    entrance === "limestone"
      ? [0x5c5661, 0x877881, 0x95807d, 14]
      : [0x474b60, 0x77778a, 0x95909c, 12];
  const half = width * 0.5;
  const rect = ([x, y, w, h]: Rect4, ink: number) => pushQuad(paths, [x, y + GROUND, w, h], ink, 1);
  const tint = ([x, y, w, h]: Rect4, ink: number, alpha: number) =>
    pushQuad(paths, [x, y + GROUND, w, h], ink, alpha);

  rect([-half, -BASE_HEIGHT, width, BASE_HEIGHT], stone);
  rect([-half, -BASE_HEIGHT, width, 7], shaft);
  tint([-half, -BASE_HEIGHT + 4, width, 3], stone, 0.5);
  tint([width * 0.23, -BASE_HEIGHT, width * 0.27, 15], 0x181c31, 0.36);
  tint([-half, -BASE_HEIGHT, 2, 15], 0xaaa3ae, 0.27);
  for (const x of windowBays(width)) {
    tint([-half + x - 1.2, -BASE_HEIGHT, 0.65, 7], 0x9a91a3, 0.29);
  }
  rect([-half - 5, -43, width + 10, 43], stone);
  tint([half * 0.62, -43, half * 0.38 + 5, 43], 0x22263c, 0.38);
  for (const [y, ext, h] of [
    [-54, 0.5, 1.15],
    [-44, 5.5, 2.4],
    [-21, 5, 1.1],
    [-3.5, 6.5, 3.5],
  ] as const) {
    tint([-half - ext, y, width + 2 * ext, h], trim, 0.72);
  }
  for (const side of [-1, 1]) {
    for (let i = 0; i < 3; i++) {
      const x = side * (portal + 5 + (i * (half - portal - 8)) / 3);
      rect([x - 2, -39, 4, 11], 0x323549);
      tint([x - 1.25, -38.5, 2.5, 7.8], GLASS, 0.35);
      rect([x - 2.65, -17, 5.3, 12.5], 0x262a3d);
      tint([x - 1.9, -16.5, 3.8, 10.7], GLASS, 0.27);
    }
  }
  const crown = entrance === "limestone" ? -31 : -38;
  rect([-portal - 2, crown, (portal + 2) * 2, -crown - 3.5], metal);
  rect([-portal, crown + 1.5, portal * 2, -crown - 5], 0x252939);
  for (const bay of [-1, 0, 1]) {
    const x = bay * (portal * 0.63);
    const pane = portal * 0.5;
    tint([x - pane * 0.5, crown + 4, pane, -crown - 11], GLASS, 0.42);
    rect([x - pane * 0.5, -13, pane, 1.1], metal);
  }
  rect([-portal - 3.5, -4.1, (portal + 3.5) * 2, 1.6], trim);
  rect([-half - 7, -1.5, width + 14, 1.5], 0x343345);
}

function buildLandmark(
  width: number,
  ink: number,
  entrance: Entrance,
  crown: (paths: ArtPath[]) => void,
): Landmark {
  const front: ArtPath[] = [];
  crown(front);
  addFacade(front, [-width * 0.5, SHAFT_TOP, width, SHAFT_HEIGHT], ink, 0);
  addFrontage(front, width, entrance, ink);
  return { tile: null, front, width };
}

let cachedLandmarks: [Landmark, Landmark] | null = null;
function getLandmarks(): [Landmark, Landmark] {
  if (cachedLandmarks) return cachedLandmarks;
  const empire = buildLandmark(90, STONE, "limestone", (paths) => {
    for (const [width, top, bottom, ink, seed] of [
      [90, 214, SHAFT_TOP, STONE, 1],
      [74, 180, 214, 0x47495f, 3],
      [57, 153, 180, 0x4c4c64, 4],
      [39, 134, 153, 0x59586e, 7],
    ] as const) {
      addFacade(paths, [-width * 0.5, top, width, bottom - top], ink, seed);
      if (width < 90) pushQuad(paths, [-width * 0.5, top, width, 1.6], 0xc6b0a8, 0.51);
    }
    for (const [width, y, height, color] of [
      [31, 127, 7, 0x777289],
      [25, 116, 11, 0x8c8293],
      [18, 105, 12, 0xa39a9c],
      [11, 96, 10, 0xa89e9e],
      [6, 84, 13, 0x95899a],
      [2.6, 64, 21, 0xb6a6a9],
    ] as const) {
      pushQuad(paths, [-width * 0.5, y, width, height], color, 1);
    }
    for (const x of [-7, -2, 3]) {
      pushQuad(paths, [x, 108, 2.1, 14], 0xdac3a8, 1);
    }
    const beacon: Point2[] = [];
    for (let i = 0; i < 16; i++) {
      const a = (i * Math.PI * 2) / 16;
      beacon.push([1.5 * Math.cos(a), 63.5 + 1.5 * Math.sin(a)]);
    }
    pushShape(paths, beacon, 0xf4a47f, 0.9);
  });

  const chrysler = buildLandmark(72, STEEL, "steel", (paths) => {
    addFacade(paths, [-36, 211, 72, SHAFT_TOP - 211], STEEL, 5);
    pushQuad(paths, [-36, 211, 72, 3], 0x77738b, 1);
    pushQuad(paths, [-25, 202, 50, 11], 0x6e738d, 1);
    for (const [rx, base, height, metal] of ARCHES) {
      const arch: Point2[] = [];
      for (let step = 0; step <= 20; step++) {
        const angle = (step * Math.PI) / 20;
        arch.push([rx * Math.cos(angle), base - height * Math.sin(angle)]);
      }
      pushShape(paths, arch, metal, 1);
      for (const x of [-rx * 0.56, 0, rx * 0.56]) {
        pushShape(
          paths,
          [
            [x - 2.5, base - 3],
            [x, base - 10],
            [x + 2.5, base - 3],
          ],
          0x43465f,
          1,
        );
      }
    }
    pushQuad(paths, [-1.1, 115, 2.2, 31], 0xb8afbc, 1);
    pushQuad(paths, [-2.6, 137, 5.2, 12], 0x9c95aa, 1);
  });

  cachedLandmarks = [empire, chrysler];
  return cachedLandmarks;
}

function getLandmarkTile(building: Landmark): CachedTile {
  if (!building.tile) {
    const baseWidth = building.width + 16;
    building.tile = createTileCanvas(
      -baseWidth * 0.5,
      54,
      baseWidth,
      GROUND - 54,
      (ctx) => {
        drawArtPaths(ctx, building.front, 1);
      },
      2,
    );
  }
  return building.tile;
}

function openingPlacement(view: View, x: number): [Point2, number] {
  const [foot, zoom] = view.groundBillboard(x, STREET_DEPTH, streetGrade(view));
  return [foot, zoom * 0.9];
}

function cityPlacement(view: View, x: number): [Point2, number] {
  const grade = streetGrade(view);
  const [[screenX, cursorY], zoom] = view.groundBillboard(x, STREET_DEPTH, grade);
  const neutral = view.neutral();
  const [[, referenceY]] = neutral.groundBillboard(x, STREET_DEPTH, grade);
  const foot = neutral.project(neutral.xAtDepth(x, LANDMARKS_DEPTH), LANDMARKS_DEPTH, grade)[1];
  return [[screenX, foot + (cursorY - referenceY)], zoom * SCALE];
}

export function landmarkDepth(inland: number): number {
  return STREET_DEPTH + (LANDMARKS_DEPTH - STREET_DEPTH) * inland;
}

export function drawMenuLandmarks(ctx: CanvasRenderingContext2D, view: View, scale: number): void {
  const [empire, chrysler] = getLandmarks();
  for (const [building, x] of [
    [empire, EMPIRE_X],
    [chrysler, CHRYSLER_X],
  ] as const) {
    const [[lx, foot], zoom] = openingPlacement(view, x);
    const tile = getLandmarkTile(building);
    ctx.save();
    ctx.translate(lx * scale, foot * scale);
    paintTile(ctx, tile, 0, -GROUND, scale * zoom);
    ctx.restore();
  }
}

export function projectLandmarks(
  original: View,
  view: View,
  camera: Camera,
  inland: number,
): ArtPath[] {
  const paths: ArtPath[] = [];
  if (camera.eye[2] >= landmarkDepth(inland)) return paths;
  const [empire, chrysler] = getLandmarks();
  for (const [building, position] of [
    [empire, EMPIRE_X],
    [chrysler, CHRYSLER_X],
  ] as const) {
    const [[oldX, oldY], oldScale] = openingPlacement(original, position);
    const [[newX, newY], newScale] = cityPlacement(original, position);
    for (const shape of building.front) {
      const pts = shape.points.map(([x, y]) => {
        const opening = original.lift(
          [oldX + x * oldScale, oldY + (y - GROUND) * oldScale],
          STREET_DEPTH,
        );
        const city = original.lift(
          [newX + x * newScale, newY + (y - GROUND) * newScale],
          LANDMARKS_DEPTH,
        );
        return mix3(opening, city, inland);
      });
      view.face(paths, pts, shape.ink, shape.opacity);
    }
  }
  return paths;
}
