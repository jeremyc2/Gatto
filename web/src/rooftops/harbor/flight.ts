import { drawArtPaths, moonPaths } from "../art";
import {
  drawSky,
  SKYLINE_ROWS,
  skylineHeightVariation,
  skylineRowTop,
  type SkylineRowSpec,
} from "../course-renderer";
import {
  clamp,
  COURSE_MOON_DIAMETER,
  remEuclid,
  rgbCss,
  SKY,
  type ArtPath,
  type Point2,
  type Point3,
  type Rect4,
} from "../types";
import { ferry, sailboat, tugboat, waterDetail } from "./boats";
import { BridgeArt } from "./bridge";
import { drawMenuLandmarks, landmarkDepth, projectLandmarks } from "./landmarks";
import {
  blendInk,
  Camera,
  compileArtMesh,
  drawCompiledMesh,
  drawHarborPaths,
  FERRY_DEPTH,
  HARBOR_HEIGHT,
  HARBOR_WIDTH,
  HORIZON,
  mix3,
  MOON_CENTER,
  MOON_DIAMETER,
  pushShape,
  SAILBOAT_DEPTH,
  shadeInk,
  TUG_DEPTH,
  View,
  WAKE_STREAKS,
  WATER,
  type Aim,
  type CompiledLayer,
} from "./projection";
import { Bank, recreation, STREET_DEPTH, streetGrade } from "./shore";

const HANDOFF = 9.65;
const LAUNCH = 0.14;
const SPEED = 2;
const SHOT_END = LAUNCH + HANDOFF / SPEED;
const FADE_OUT = 0.12;
const CUT = SHOT_END + FADE_OUT;
const ROUTE_END = HANDOFF + FADE_OUT * SPEED;
const BLACK_BEAT = 0.02;
const FADE_IN = 0.1;
const CITY_REVEAL_DEPTH = -550;
const POCKET_REVEAL_DEPTH = -660;
const POCKET_OPENS_BY = 330;
const HAZE_INK = 0x121525;
const GROUND_FLOOR = 25;
const LAND_BAND = 230;
const FLIGHT_MOON_DIAMETER = COURSE_MOON_DIAMETER * 1.3;
const MENU_ROW_DEPTHS = [725, 565, 400] as const;

interface CityRow {
  readonly depth: number;
  readonly spacing: number;
  readonly offset: number;
  readonly heights: readonly [number, number];
  readonly ink: number;
}

function cityRowFromSkyline(
  depth: number,
  heights: readonly [number, number],
  row: SkylineRowSpec,
): CityRow {
  return {
    depth,
    spacing: row.spacing,
    offset: -145 * row.parallax,
    heights,
    ink: row.ink,
  };
}

const HORIZON_VARIATIONS = [8, 34, 16, 39, 21, 0, 27, 12, 31, 3, 24] as const;

function cityRowHeight(row: CityRow, index: number, layer: number, horizonRow: boolean): number {
  const variation = horizonRow
    ? HORIZON_VARIATIONS[remEuclid(index, 11)]! / 39
    : skylineHeightVariation(index, layer) / 129;
  return row.heights[0] + (row.heights[1] - row.heights[0]) * variation;
}

const SHORE_ROWS: readonly CityRow[] = [
  cityRowFromSkyline(1240, [86, 193], SKYLINE_ROWS[0]!),
  cityRowFromSkyline(940, [73, 156], SKYLINE_ROWS[1]!),
  cityRowFromSkyline(580, [58, 128], SKYLINE_ROWS[2]!),
];

const DISTANT_ROWS: readonly CityRow[] = [
  { depth: 3500, spacing: 61, offset: 39.6, heights: [164, 242], ink: 0x191e31 },
  { depth: 3100, spacing: 106, offset: 58.6, heights: [159, 247], ink: 0x191d30 },
  { depth: 2740, spacing: 134, offset: 71.8, heights: [137, 243], ink: 0x1d2035 },
  { depth: 2340, spacing: 113, offset: 85, heights: [119, 211], ink: 0x1c2033 },
  { depth: 1970, spacing: 98, offset: 105, heights: [82, 159], ink: 0x202237 },
  { depth: 1620, spacing: 117, offset: 121, heights: [67, 146], ink: 0x212338 },
];

const ROUTE: readonly (readonly [number, number, number, number])[] = [
  [0, 0, 0, 0],
  [0.7, -1170, 115, 0.021],
  [1.42, -650, 286, 0.04],
  [1.8, -430, 362, 0.046],
  [2.02, -295, 393, 0.03],
  [2.44, -125, 415, -0.022],
  [2.79, -32, 418, -0.075],
  [3.24, 91, 410, -0.129],
  [3.88, 226, 392, -0.164],
  [4.64, 370, 362, -0.162],
  [5.35, 537, 310, -0.146],
  [6.13, 820, 256, -0.139],
  [6.97, 1140, 206, -0.142],
  [7.63, 1430, 166, -0.147],
  [8.23, 1710, 142, -0.15],
  [8.85, 2000, 122, -0.154],
  [9.28, 2180, 114, -0.161],
  [HANDOFF, 2310, 110, -0.167],
  [ROUTE_END, 2420, 105, -0.173],
];

export function smooth(t: number): number {
  const c = clamp(t, 0, 1);
  return c * c * (3 - 2 * c);
}

let cachedHarborMoon: ArtPath[] | null = null;
function getHarborMoonPaths(): ArtPath[] {
  if (!cachedHarborMoon) {
    cachedHarborMoon = moonPaths(
      MOON_CENTER[0] - MOON_DIAMETER * 0.5,
      MOON_CENTER[1] - MOON_DIAMETER * 0.5,
      MOON_DIAMETER,
      0,
    );
  }
  return cachedHarborMoon;
}

export function harborFrame(
  viewportW: number,
  viewportH: number,
): { x: number; y: number; width: number; height: number; scale: number } {
  const scale = Math.max(0.01, Math.min(viewportW / HARBOR_WIDTH, viewportH / HARBOR_HEIGHT));
  const width = HARBOR_WIDTH * scale;
  const height = HARBOR_HEIGHT * scale;
  return {
    x: (viewportW - width) * 0.5,
    y: (viewportH - height) * 0.5,
    width,
    height,
    scale,
  };
}

export function harborMenuFrame(
  viewportW: number,
  viewportH: number,
): ReturnType<typeof harborFrame> {
  const frame = harborFrame(viewportW, viewportH);
  // In portrait, keep a continuous sky above the harbor and water below it.
  if (viewportH > viewportW) {
    frame.y = Math.max(frame.y, viewportH - frame.height - Math.min(110, viewportH * 0.14));
  }
  return frame;
}

export class Flight {
  static readonly DURATION = CUT + BLACK_BEAT + FADE_IN;
  static readonly LAUNCH = LAUNCH;
  static readonly SHOT_END = SHOT_END;
  static readonly CUT = CUT;

  time = 0;

  shotTime(): number {
    return clamp((this.time - LAUNCH) * SPEED, 0, ROUTE_END);
  }

  advance(elapsed: number): boolean {
    this.time += elapsed;
    return this.time >= Flight.DURATION;
  }

  revealsCourse(): boolean {
    return this.time >= CUT;
  }

  titleFadeOpacity(): number {
    const titleFade = LAUNCH + 0.04;
    if (this.time >= titleFade) return 0;
    return 1 - smooth(this.time / titleFade);
  }

  veilOpacity(): number {
    if (this.revealsCourse()) {
      return 1 - smooth((this.time - CUT - BLACK_BEAT) / FADE_IN);
    }
    return smooth((this.time - SHOT_END) / FADE_OUT);
  }

  camera(view: View): Camera {
    const rest = Camera.atMenu(view);
    const route = ROUTE.map(
      (row) => [row[0], row[1], row[2], row[3]] as [number, number, number, number],
    );
    route[0]![1] = rest.eye[2];
    route[0]![2] = rest.eye[1];

    const shotTime = this.shotTime();
    let i = 0;
    while (i < route.length && route[i]![0] <= shotTime) i += 1;
    i = clamp(i, 1, route.length - 1);

    const a = route[i - 1]!;
    const b = route[i]!;
    const p = route[Math.max(0, i - 2)]!;
    const q = route[Math.min(route.length - 1, i + 1)]!;
    const span = b[0] - a[0];
    const t = clamp((shotTime - a[0]) / span, 0, 1);
    const h = smooth(t);

    const track = (j: 1 | 2 | 3): number => {
      if (j === 3 && a[j] === b[j]) return a[j];
      const v0 = i === 1 || (j === 3 && p[j] === a[j]) ? 0 : ((b[j] - p[j]) * span) / (b[0] - p[0]);
      const v1 = j === 3 && b[j] === q[j] ? 0 : ((q[j] - a[j]) * span) / (q[0] - a[0]);
      return a[j] * (1 - h) + b[j] * h + (v0 * (1 - t) - v1 * t) * t * (1 - t);
    };

    const z = track(1);
    return rest.dolly([view.xAtDepth(HARBOR_WIDTH * 0.5, z), track(2), z], track(3));
  }
}

export function skylineCamera(original: View, camera: Camera): Camera {
  const COURT = 535;
  const CITY = 1660;
  const LAG = 650;
  const travel = camera.eye[2];
  const opening = smooth((travel - POCKET_REVEAL_DEPTH) / (POCKET_OPENS_BY - POCKET_REVEAL_DEPTH));
  const lag =
    travel < POCKET_OPENS_BY
      ? LAG * opening
      : LAG * (1 - smooth((travel - COURT) / (CITY - COURT)));
  const compact = smooth((travel + 1440) / 480) * (1 - opening);
  const z = travel - lag;
  return camera.withEye([
    original.xAtDepth(HARBOR_WIDTH * 0.5, z),
    camera.eye[1] - 30 * compact,
    z,
  ]);
}

function cityRelief(depth: number): number {
  return smooth((depth - CITY_REVEAL_DEPTH) / 175);
}

function horizonVisibility(cameraDepth: number, rowDepth: number): number {
  const clearTo = 1130 + Math.max(0, cameraDepth - 125) * 1.36;
  return smooth((clearTo - rowDepth) / 510);
}

function pocketDepth(depth: number): number {
  return smooth((depth - POCKET_REVEAL_DEPTH) / (135 - POCKET_REVEAL_DEPTH));
}

function windowRun(span: number, pane: number, spacing: number, margin: number): number[] {
  const room = span - 2 * margin - pane;
  const count = room >= 0 ? 1 + Math.floor(room / spacing) : 0;
  const start = (span - pane - Math.max(0, count - 1) * spacing) * 0.5;
  const out: number[] = [];
  for (let i = 0; i < count; i++) out.push(start + i * spacing);
  return out;
}

function groundFloor(
  index: number,
  width: number,
  wall: number,
  detail: number,
  panel: (rect: Rect4, color: number) => void,
): void {
  const center = width * 0.5;
  const base = shadeInk(wall, 0.9);
  const recess = shadeInk(wall, 0.68);
  const glass = blendInk(recess, 0x655865, 0.37 * detail);
  const trim = blendInk(base, 0x665867, 0.39 * detail);
  panel([0, -GROUND_FLOOR, width, GROUND_FLOOR], base);
  panel([0, -GROUND_FLOOR, width, 1.6], trim);
  const kind = remEuclid(index, 3);
  if (kind === 0) {
    panel([center - 11, -18, 22, 18], recess);
    panel([center - 14, -19.3, 28, 2.3], trim);
    panel([center - 8.4, -14, 7.6, 10], glass);
    panel([center + 0.8, -14, 7.6, 10], glass);
  } else if (kind === 1) {
    panel([13, -17, width - 26, 17], recess);
    panel([12, -18.5, width - 24, 2.5], trim);
    const pane = (width - 42) * 0.5;
    panel([16, -12, pane, 8], glass);
    panel([26 + pane, -12, pane, 8], glass);
  } else {
    panel([center - 6, -16, 12, 16], recess);
    panel([center - 4.2, -12.4, 8.4, 4], glass);
    panel([center - 8, -1.4, 16, 1.4], trim);
  }
}

function buildCity(
  original: View,
  view: View,
  camera: Camera,
  relief: number,
  clip: Rect4,
  inland: number,
): [ArtPath[], ArtPath[]] {
  const buildings: [ArtPath[], ArtPath[]] = [[], []];
  const grade = streetGrade(original);
  const [leftEdge, , rightEdge] = clip;

  const allRows: [number, CityRow][] = [];
  for (let i = 0; i < DISTANT_ROWS.length; i++) {
    allRows.push([i + SHORE_ROWS.length, DISTANT_ROWS[i]!]);
  }
  for (let i = 0; i < SHORE_ROWS.length; i++) {
    allRows.push([i, SHORE_ROWS[i]!]);
  }

  for (const [layer, row] of allRows) {
    const distant = layer >= SHORE_ROWS.length;
    const paintedDepth = MENU_ROW_DEPTHS[layer];
    const open = paintedDepth ?? row.depth;
    const distance = open + (row.depth - open) * inland;
    const paths = buildings[distance < landmarkDepth(inland) ? 1 : 0];
    const horizonRow = layer === SHORE_ROWS.length;
    const visibility = distant ? horizonVisibility(camera.eye[2], distance) : 1;
    if (visibility === 0) continue;

    const ink = row.ink;
    const fogged = (c: number) => blendInk(HAZE_INK, c, visibility);
    const wall = fogged(ink);
    const surface = (light: number) => fogged(shadeInk(ink, 1 + relief * (light - 1)));
    const heading = original.xAtDepth(HARBOR_WIDTH * 0.5, distance);
    const first = Math.floor((leftEdge - row.offset) / row.spacing) - 3;
    const last = Math.ceil((rightEdge - row.offset) / row.spacing) + 3;
    const width = row.spacing - (horizonRow ? 3 : 9);
    const center = Math.floor((HARBOR_WIDTH * 0.5 - row.offset) / row.spacing);

    const indices: number[] = [];
    const leftEnd = Math.min(last, center - 1);
    for (let idx = first; idx <= leftEnd; idx++) indices.push(idx);
    const rightStart = Math.max(first, center + 1);
    for (let idx = last; idx >= rightStart; idx--) indices.push(idx);
    if (center >= first && center <= last) indices.push(center);

    const skylineSpec = SKYLINE_ROWS[layer];

    for (const index of indices) {
      const left = index * row.spacing + row.offset;
      const roofStyle = remEuclid(index, 5);
      const height = cityRowHeight(row, index, layer, horizonRow);
      const top = grade - height;
      const FLOOR_SCALE = 0.86;
      const rise = (offset: number) => top + offset * FLOOR_SCALE;
      const across = (offset: number) => left + offset;

      let painted: [number, number, number] | null = null;
      if (skylineSpec) {
        const lean = layer === 0 ? 290 : layer === 1 ? 219 : 160;
        const [[px, foot], zoom] = original.billboard(left, lean, original.horizon);
        const pTop = foot + (skylineRowTop(skylineSpec, index, layer) - original.horizon) * zoom;
        painted = [px, Math.min(pTop, original.horizon), zoom];
      }

      const point = ([x, y]: Point2): Point3 => {
        const world: Point3 = [original.xAtDepth(x, row.depth), y, row.depth];
        if (!painted) return world;
        const [oldX, oldTop, zoom] = painted;
        const oldY =
          y < top
            ? oldTop + ((y - top) * zoom) / FLOOR_SCALE
            : oldTop + ((original.horizon - oldTop) * (y - top)) / height;
        const art = original.lift([oldX + (x - left) * zoom, oldY], open);
        return mix3(art, world, inland);
      };

      const a = point([left, top]);
      const b = point([across(width), top]);
      const c = point([across(width), grade]);
      const d = point([left, grade]);
      const bounds: Point3[] = [a, b, c, d];
      const extent = 87 + remEuclid(index * 13 + layer * 7, 31);
      const blockDepth = extent * relief;
      const back = (p: Point3) => original.rear(p, blockDepth);

      const front = view.polygon(bounds);
      if (
        front.length === 0 &&
        (relief === 0 ||
          (view.polygon([back(a), back(b), back(c), back(d)]).length === 0 &&
            view.polygon([a, back(a), back(d), d]).length === 0 &&
            view.polygon([b, back(b), back(c), c]).length === 0 &&
            view.polygon([a, b, back(b), back(a)]).length === 0))
      ) {
        continue;
      }

      const [paneWidth, paneHeight, columnStep, floorStep] = horizonRow
        ? roofStyle === 0 || roofStyle === 2
          ? [4.5, 9.5, 12.5, 20]
          : roofStyle === 1 || roofStyle === 3
            ? [6.5, 7, 15, 19]
            : [4, 8.5, 14, 22]
        : [6, 8.6, 20, 23];

      const rows = windowRun(height - GROUND_FLOOR, paneHeight, floorStep, 9);
      const phase = remEuclid(index, 13);
      const glow = distant || layer === 0 ? 0.13 : 0.23;

      const drawWindows = (
        span: number,
        wallInk: number,
        light: number,
        at: (p: Point2) => Point3,
      ) => {
        const columns = windowRun(span, paneWidth, columnStep, 9);
        const palette = [
          fogged(blendInk(wallInk, 0xd9b49b, 0.34 * glow * light)),
          fogged(blendInk(wallInk, 0xd9b49b, 0.58 * glow * light)),
          fogged(blendInk(wallInk, 0xd9b49b, 0.89 * glow * light)),
        ] as const;
        const panes: [ArtPath[], ArtPath[], ArtPath[]] = [[], [], []];
        for (let floor = 0; floor < rows.length; floor++) {
          const wy = rows[floor]!;
          for (let col = 0; col < columns.length; col++) {
            const wx = columns[col]!;
            const colorIdx = ((phase * 11 + floor * 7 + col * (5 + floor) + col * col) % 13) % 3;
            const r = wx + paneWidth;
            const bot = wy + paneHeight;
            view.face(
              panes[colorIdx]!,
              [at([wx, wy]), at([r, wy]), at([r, bot]), at([wx, bot])],
              palette[colorIdx]!,
              1,
            );
          }
        }
        paths.push(...panes[0], ...panes[1], ...panes[2]);
      };

      const frontAt = ([x, y]: Point2) => point([across(x), top + y]);

      if (relief > 0) {
        if (camera.eye[2] > distance + blockDepth) {
          view.face(paths, [back(a), back(b), back(c), back(d)], wall, 1);
          drawWindows(width, ink, 0.72, (p) => back(frontAt(p)));
        }
        let side: [Point3, Point3, number] | null = null;
        if (heading < a[0]) side = [a, d, 0.86];
        else if (heading > b[0]) side = [b, c, 0.78];
        if (side) {
          const [topEdge, foot, light] = side;
          const poly = view.polygon([topEdge, back(topEdge), back(foot), foot]);
          if (poly.length >= 3) {
            const sideInk = shadeInk(ink, 1 + relief * (light - 1));
            pushShape(paths, poly, fogged(sideInk), 1);
            drawWindows(extent, sideInk, relief * 0.62, ([wx, wy]) =>
              original.rear(mix3(topEdge, foot, wy / height), wx * relief),
            );
          }
        }
        if (camera.eye[1] < (a[1] + b[1]) * 0.5) {
          view.face(paths, [a, b, back(b), back(a)], surface(1.32), 1);
        }
      }

      const faceVisible = camera.eye[2] <= distance;
      if (faceVisible && front.length >= 3) {
        pushShape(paths, front, wall, 1);
      }

      const setback = ([sLeft, sRight, sTop, sBottom]: Rect4, length: number) => {
        const sa = point([across(sLeft), rise(sTop)]);
        const sb = point([across(sRight), rise(sTop)]);
        const sc = point([across(sRight), rise(sBottom)]);
        const sd = point([across(sLeft), rise(sBottom)]);
        const rear = [
          original.rear(sa, blockDepth * length),
          original.rear(sb, blockDepth * length),
          original.rear(sc, blockDepth * length),
          original.rear(sd, blockDepth * length),
        ] as const;
        if (relief > 0) {
          if (heading < sa[0]) {
            view.face(paths, [sa, rear[0], rear[3], sd], surface(0.84), 1);
          } else if (heading > sb[0]) {
            view.face(paths, [sb, rear[1], rear[2], sc], surface(0.83), 1);
          }
          if (camera.eye[1] < (sa[1] + sb[1]) * 0.5) {
            view.face(paths, [sa, sb, rear[1], rear[0]], surface(1.44), 1);
          }
          if (camera.eye[2] >= rear[0][2]) {
            view.face(paths, [rear[0], rear[1], rear[2], rear[3]], wall, 1);
          }
        }
        if (faceVisible) {
          view.face(paths, [sa, sb, sc, sd], wall, 1);
        }
      };

      if (horizonRow) {
        if (roofStyle === 0) {
          setback([8, 24, -7, 1], 0.56);
          setback([11, 16, -25, -6], 0.28);
          setback([9.5, 17.5, -27, -24], 0.32);
        } else if (roofStyle === 1) {
          setback([7, 51, -8, 1], 0.7);
          setback([25, 46, -19, -7], 0.52);
        } else if (roofStyle === 2) {
          setback([13, 45, -7, 1], 0.66);
          setback([20, 34, -24, -6], 0.44);
          setback([28, 30, -40, -23], 0.2);
        } else if (roofStyle === 3) {
          setback([6, 28, -11, 1], 0.66);
          setback([38, 52, -5, 1], 0.4);
        }
      } else {
        setback([13, row.spacing - 22, -9, 1], 0.79);
        if (index % 3 === 0) {
          setback([35, 56, -30, -6], 0.49);
          setback([41, 43, -46, -29], 0.21);
        }
      }

      if (!faceVisible) continue;
      if (!distant && inland > 0) {
        const detail = smooth(1 - (distance - camera.eye[2] - 320) / 700);
        groundFloor(index, width, ink, detail, ([gx, gy, gw, gh], color) => {
          const px = left + gx;
          const py = grade + gy;
          view.face(
            paths,
            [
              point([px, py]),
              point([px + gw, py]),
              point([px + gw, py + gh]),
              point([px, py + gh]),
            ],
            fogged(blendInk(ink, color, inland)),
            1,
          );
        });
      }
      drawWindows(width, ink, 1, frontAt);
    }
  }
  return buildings;
}

function flightStreets(view: View, camera: Camera, clarity: number): ArtPath[] {
  const paths: ArtPath[] = [];
  const grade = streetGrade(view);
  for (let band = 20; band >= 0; band--) {
    const near = STREET_DEPTH + band * LAND_BAND;
    const distance = Math.max(0, near - Math.max(camera.eye[2], STREET_DEPTH));
    const amount = smooth(1 - distance / 2600);
    const haze = blendInk(SKY, HAZE_INK, smooth(amount * 4));
    const ink = blendInk(haze, 0x222636, amount);
    view.face(
      paths,
      [
        [-100000, grade, near],
        [100000, grade, near],
        [100000, grade, near + LAND_BAND],
        [-100000, grade, near + LAND_BAND],
      ],
      ink,
      clarity,
    );
  }
  return paths;
}

function skylineStreetY(original: View, camera: Camera, screenX: number): number | null {
  const depth = SHORE_ROWS[1]!.depth;
  if (camera.eye[2] >= depth) return null;
  const grade = streetGrade(original);
  const a = camera.point([original.xAtDepth(0, depth), grade, depth]);
  const b = camera.point([original.xAtDepth(HARBOR_WIDTH, depth), grade, depth]);
  if (!a || !b) return null;
  return a[1] + ((b[1] - a[1]) * (screenX - a[0])) / (b[0] - a[0]);
}

export function moonProjection(original: View, camera: Camera): [Point2, number] | null {
  const rest = Camera.atMenu(original);
  const start = rest.eye[2];
  const finish = ROUTE[ROUTE.length - 2]![1];
  const growth = FLIGHT_MOON_DIAMETER / MOON_DIAMETER;
  const depth = finish + (finish - start) / (growth - 1);
  const scale = Math.min((depth - start) / (depth - camera.eye[2]), growth);
  const anchor = original.lift(MOON_CENTER, 0);
  const projected = camera.point([
    camera.eye[0] + (anchor[0] - rest.eye[0]),
    camera.eye[1] + (anchor[1] - rest.eye[1]),
    camera.eye[2] + (anchor[2] - rest.eye[2]),
  ]);
  if (!projected) return null;
  const climb = smooth((camera.eye[2] - 100) / (finish - 100));
  const x = projected[0] - 18 * climb;
  let y = MOON_CENTER[1] + (projected[1] - MOON_CENTER[1]) * 0.34 - 44 * climb;
  const radius = MOON_DIAMETER * scale * 0.5;
  const skyline = skylineCamera(original, camera);
  const streetY = skylineStreetY(original, skyline, x);
  if (streetY !== null) {
    const limit = streetY - radius - 8;
    const blend = Math.max(0, 22 - Math.abs(limit - y)) / 22;
    y = Math.min(limit, y) - blend * blend * 5.5;
  }
  return [[x, y], scale];
}

function flightBoats(original: View, view: View, time: number, depth: number): ArtPath[] {
  const art =
    depth === TUG_DEPTH
      ? tugboat(original, time)
      : depth === FERRY_DEPTH
        ? ferry(original, time)
        : sailboat(original, time);
  const paths: ArtPath[] = [];
  const eye = Camera.atMenu(original).eye;
  for (let idx = 0; idx < art.length; idx++) {
    const s = art[idx]!;
    const pts = s.points.map((pt) => {
      const at = original.lift(pt, depth);
      if (idx < WAKE_STREAKS) {
        const t = (original.waterline() - eye[1]) / (at[1] - eye[1]);
        return mix3(eye, at, t);
      }
      return at;
    });
    view.face(paths, pts, s.ink, s.opacity);
  }
  return paths;
}

function flightWater(original: View, view: View, time: number, clip: Rect4): ArtPath[] {
  const paths: ArtPath[] = [];
  const y = view.waterline();
  view.face(
    paths,
    [
      [-100000, y, -100000],
      [100000, y, -100000],
      [100000, y, 152],
      [-100000, y, 152],
    ],
    WATER,
    1,
  );
  const eye = Camera.atMenu(original).eye;
  const onWater = (screen: Point2): Point3 => {
    const at = original.lift(screen, 0);
    const t = (y - eye[1]) / (at[1] - eye[1]);
    return mix3(eye, at, t);
  };
  const leftEdge = Math.min(0, clip[0] - 80);
  const rightEdge = Math.max(original.width, clip[2] + 80);
  waterDetail(
    original,
    HARBOR_HEIGHT,
    time,
    ([rx, ry, rw, rh], ink, opacity) => {
      view.face(
        paths,
        [
          onWater([rx, ry]),
          onWater([rx + rw, ry]),
          onWater([rx + rw, ry + rh]),
          onWater([rx, ry + rh]),
        ],
        ink,
        opacity,
      );
    },
    leftEdge,
    rightEdge,
  );
  return paths;
}

function flightRecreation(
  view: View,
  camera: Camera,
  cityCam: Camera,
  inland: number,
): [ArtPath[], ArtPath[]] {
  const layer = SHORE_ROWS.length - 1;
  const open = MENU_ROW_DEPTHS[layer]!;
  const depth = SHORE_ROWS[layer]!.depth;
  let frontage: [Point2, Point2] | null = null;
  if (cityCam.eye[2] < open + (depth - open) * inland) {
    const liftEdge = (x: number) => {
      const painted = view.lift([x, view.horizon], open);
      const street: Point3 = [view.xAtDepth(x, depth), streetGrade(view), depth];
      return cityCam.point(mix3(painted, street, inland));
    };
    const left = liftEdge(0);
    const right = liftEdge(HARBOR_WIDTH);
    if (left && right) frontage = [left, right];
  }
  return recreation(view, camera.eye[2], frontage);
}

export function menuTitleLayout(
  viewportW: number,
  viewportH: number,
  touchMode: boolean,
): {
  titleY: number;
  titleSize: number;
  actionsTop: number;
  scale: number;
  frameX: number;
  frameW: number;
} {
  const frame = harborFrame(viewportW, viewportH);
  const portrait = viewportH > viewportW;
  const shortLandscape = !portrait && viewportH < 480;

  if (portrait) {
    const titleSize = clamp(viewportW * 0.19, 52, 92);
    const titleY = Math.max(88, viewportH * 0.16);
    const actionsTop = titleY + titleSize * 0.62 + 18;
    return {
      titleY,
      titleSize,
      actionsTop,
      scale: frame.scale,
      frameX: 0,
      frameW: viewportW,
    };
  }

  if (shortLandscape || (touchMode && frame.scale < 0.85)) {
    const titleSize = clamp(viewportH * 0.17, 42, 72);
    const titleY = Math.max(36, viewportH * 0.135);
    const actionsTop = titleY + titleSize * 0.54 + 8;
    return {
      titleY,
      titleSize,
      actionsTop,
      scale: frame.scale,
      frameX: frame.x,
      frameW: frame.width,
    };
  }

  return {
    titleY: frame.y + frame.height * 0.115 + 66 * frame.scale,
    titleSize: 119 * frame.scale,
    actionsTop: frame.y + frame.height * 0.115 + (132 + 14) * frame.scale,
    scale: frame.scale,
    frameX: frame.x,
    frameW: frame.width,
  };
}

export function drawHarborTitle(
  ctx: CanvasRenderingContext2D,
  viewportW: number,
  viewportH: number,
  touchMode: boolean,
  opacity: number,
): void {
  if (opacity <= 0.001) return;
  const layout = menuTitleLayout(viewportW, viewportH, touchMode);
  ctx.save();
  ctx.globalAlpha = opacity;
  ctx.fillStyle = rgbCss(0xffe1b1);
  ctx.font = `700 ${layout.titleSize}px Georgia, "Times New Roman", serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("Rooftops", viewportW * 0.5, layout.titleY);
  ctx.restore();
}

export class HarborMenuRenderer {
  protected readonly bridge = new BridgeArt();
  private skylineKey = "";
  private skylineMesh: CompiledLayer[] = [];

  private getOpeningSkyline(original: View, clip: Rect4): CompiledLayer[] {
    const key = `${original.aim.x}:${original.aim.y}:${Math.round(clip[0])}:${Math.round(clip[2])}`;
    if (key !== this.skylineKey) {
      this.skylineKey = key;
      const camera = Camera.atMenu(original).withClip(clip);
      const view = original.withCamera(camera);
      const [paths] = buildCity(original, view, camera, 0, clip, 0);
      this.skylineMesh = compileArtMesh(paths);
    }
    return this.skylineMesh;
  }

  renderMenu(
    ctx: CanvasRenderingContext2D,
    viewportW: number,
    viewportH: number,
    time: number,
    aim: Aim,
    touchMode = false,
    titleOpacity = 1,
  ): void {
    ctx.fillStyle = rgbCss(SKY);
    ctx.fillRect(0, 0, viewportW, viewportH);

    const frame = harborMenuFrame(viewportW, viewportH);
    const padX = frame.x / frame.scale;
    const padTop = frame.y / frame.scale;
    const padBottom = Math.max(0, (viewportH - (frame.y + frame.height)) / frame.scale);
    const clip: Rect4 = [-padX, -padTop, HARBOR_WIDTH + padX, HARBOR_HEIGHT + padBottom];
    const view = new View(HARBOR_WIDTH, HARBOR_HEIGHT * HORIZON, aim);
    const bank = new Bank(view);

    ctx.save();
    ctx.translate(frame.x, frame.y);
    this.renderMenuScenery(ctx, view, bank, time, frame.scale, clip, padX, padBottom);
    ctx.restore();
    drawHarborTitle(ctx, viewportW, viewportH, touchMode, titleOpacity);
  }

  protected renderMenuScenery(
    ctx: CanvasRenderingContext2D,
    view: View,
    bank: Bank,
    time: number,
    scale: number,
    clip: Rect4,
    padX: number,
    padBottom: number,
  ): void {
    const [clipLeft, clipTop, clipRight, clipBottom] = clip;
    ctx.save();
    ctx.translate(clipLeft * scale, clipTop * scale);
    drawSky(ctx, clipRight - clipLeft, clipBottom - clipTop, time, scale, 145);
    ctx.restore();

    drawArtPaths(ctx, getHarborMoonPaths(), scale);
    drawCompiledMesh(ctx, this.getOpeningSkyline(view, clip), scale);
    drawHarborPaths(ctx, bank.inland(), scale);
    drawMenuLandmarks(ctx, view, scale);

    const waterBottom = Math.max(HARBOR_HEIGHT, clipBottom);
    drawHarborPaths(ctx, bank.water(waterBottom), scale);
    waterDetail(
      view,
      HARBOR_HEIGHT,
      time,
      ([rx, ry, rw, rh], ink, alpha) => {
        ctx.globalAlpha = alpha;
        ctx.fillStyle = rgbCss(ink);
        ctx.fillRect(rx * scale, ry * scale, rw * scale, rh * scale);
      },
      clipLeft,
      clipRight,
    );
    if (padBottom > 0) {
      ctx.fillStyle = rgbCss(0x080d20);
      ctx.globalAlpha = 7 * 0.058;
      ctx.fillRect(
        clipLeft * scale,
        HARBOR_HEIGHT * scale,
        (clipRight - clipLeft) * scale,
        padBottom * scale,
      );
    }
    ctx.globalAlpha = 1;

    drawHarborPaths(ctx, bank.shore(), scale);
    drawHarborPaths(ctx, tugboat(view, time), scale);
    this.bridge.render(ctx, view, view.deck(), time, scale, padX + 260);
    drawHarborPaths(ctx, ferry(view, time), scale);
    drawHarborPaths(ctx, sailboat(view, time), scale);
  }
}

export class HarborRenderer extends HarborMenuRenderer {
  renderFlight(
    ctx: CanvasRenderingContext2D,
    viewportW: number,
    viewportH: number,
    flight: Flight,
    menuTime: number,
    aim: Aim,
    touchMode = false,
  ): void {
    const frame = harborFrame(viewportW, viewportH);
    const original = new View(HARBOR_WIDTH, HARBOR_HEIGHT * HORIZON, aim);
    const padX = frame.x / frame.scale;
    const padY = frame.y / frame.scale;
    const clip: Rect4 = [-padX, -padY, HARBOR_WIDTH + padX, HARBOR_HEIGHT + padY];
    const camera = flight.camera(original).withClip(clip);
    const time = menuTime + flight.time;

    ctx.fillStyle = rgbCss(SKY);
    ctx.fillRect(0, 0, viewportW, viewportH);

    ctx.save();
    ctx.translate(frame.x, frame.y);
    if (flight.time <= LAUNCH) {
      this.renderMenuScenery(
        ctx,
        original,
        new Bank(original),
        time,
        frame.scale,
        clip,
        padX,
        padY,
      );
    } else {
      this.renderFlightHarbor(ctx, original, camera, time, frame.scale, clip);
    }
    ctx.restore();

    const titleOpacity = flight.titleFadeOpacity();
    if (titleOpacity > 0) {
      drawHarborTitle(ctx, viewportW, viewportH, touchMode, titleOpacity);
    }

    const veil = flight.veilOpacity();
    if (veil > 0) {
      ctx.fillStyle = "#000000";
      ctx.globalAlpha = veil;
      ctx.fillRect(0, 0, viewportW, viewportH);
      ctx.globalAlpha = 1;
    }
  }

  private renderFlightHarbor(
    ctx: CanvasRenderingContext2D,
    original: View,
    camera: Camera,
    time: number,
    scale: number,
    clip: Rect4,
  ): void {
    const view = original.withCamera(camera);
    const bank = new Bank(view);
    const inland = pocketDepth(camera.eye[2]);
    const cityCam = skylineCamera(original, camera);
    const cityView = original.withCamera(cityCam);
    const [farther, nearer] = buildCity(
      original,
      cityView,
      cityCam,
      cityRelief(camera.eye[2]) * inland,
      clip,
      inland,
    );
    const [surfaces, fixtures] = flightRecreation(view, camera, cityCam, inland);

    const [cx, cy, right, bottom] = clip;
    ctx.save();
    ctx.translate(cx * scale, cy * scale);
    drawSky(ctx, right - cx, bottom - cy, time, scale, 145);
    ctx.restore();

    if (inland > 0) {
      drawHarborPaths(ctx, flightStreets(view, camera, inland), scale);
    }

    const moonProj = moonProjection(original, camera);
    if (moonProj) {
      const [center, zoom] = moonProj;
      ctx.save();
      ctx.translate(center[0] * scale, center[1] * scale);
      ctx.scale(zoom, zoom);
      ctx.translate(-MOON_CENTER[0] * scale, -MOON_CENTER[1] * scale);
      drawArtPaths(ctx, getHarborMoonPaths(), scale);
      ctx.restore();
    }

    drawHarborPaths(ctx, surfaces, scale);
    drawHarborPaths(ctx, farther, scale);
    drawHarborPaths(ctx, projectLandmarks(original, cityView, cityCam, inland), scale);
    drawHarborPaths(ctx, nearer, scale);
    drawHarborPaths(ctx, fixtures, scale);
    drawHarborPaths(ctx, flightWater(original, view, time, clip), scale);
    drawHarborPaths(ctx, bank.inland(), scale);
    drawHarborPaths(ctx, bank.shore(), scale);
    drawHarborPaths(ctx, flightBoats(original, view, time, TUG_DEPTH), scale);
    this.bridge.render(ctx, view, view.deck(), time, scale, Math.max(0, -cx) + 260);
    drawHarborPaths(ctx, flightBoats(original, view, time, FERRY_DEPTH), scale);
    drawHarborPaths(ctx, flightBoats(original, view, time, SAILBOAT_DEPTH), scale);
  }
}
