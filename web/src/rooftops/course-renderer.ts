import kittenCoinUrl from "../../../packaging/kitten-coin.png";
import {
  checkpointShapes,
  drawArtPaths,
  drawPosterShape,
  homeRoofPaths,
  moonPaths,
  restingCatShapes,
  shelterShapes,
  sleepPaths,
  statPaths,
} from "./art";
import {
  apartmentRows,
  apartmentWindowX,
  checkpointPaletteFor,
  chimneyColorsFor,
  createApartments,
  FacadeArt,
  neighborhoodForLevel,
  neighborhoodTitle,
} from "./neighborhood";
import { createTileCanvas, paintTile, RoomArt, type CachedTile } from "./rooms";
import {
  arrivalFlamesOn,
  courseProgress,
  enemyPosition,
  LevelSlide,
  ROOFS,
  Run,
} from "./simulation";
import {
  ARRIVAL_DURATION,
  ARRIVAL_FLAME_OUT,
  ARRIVAL_LANDING,
  ARRIVAL_UNSTRAP,
  clamp,
  COURSE_MOON_DIAMETER,
  GOLD,
  LEVELS,
  PLAYER_AHEAD_GREEN,
  PLAYER_BEHIND_RED,
  POSTER_FONT,
  PURPLE,
  remEuclid,
  rgbCss,
  SKY,
  SPEED,
  START_X,
  timeLabel,
  type ArtPath,
  type CheckpointPalette,
  type Enemy,
  type Frame,
  type GhostGap,
  type Pose,
  type Score,
  type StatIcon,
} from "./types";

const TAU = Math.PI * 2;
const MASK64 = 0xffffffffffffffffn;

export interface SkylineRowSpec {
  readonly parallax: number;
  readonly spacing: number;
  readonly base: number;
  readonly ink: number;
}

export const SKYLINE_ROWS: readonly SkylineRowSpec[] = [
  { parallax: 0.12, spacing: 112, base: 365, ink: 0x1b2034 },
  { parallax: 0.26, spacing: 98, base: 440, ink: 0x24263d },
  { parallax: 0.43, spacing: 125, base: 510, ink: 0x2c2b43 },
];

export function skylineHeightVariation(index: number, layer: number): number {
  return remEuclid(index * 73 + layer * 41, 130);
}

export function skylineRowTop(row: SkylineRowSpec, index: number, layer: number): number {
  return row.base - 55 - skylineHeightVariation(index, layer);
}

interface StripStar {
  offset: number;
  y: number;
  size: number;
  phase: number;
}

const STRIP_WIDTH = 160;
const STARS_PER_STRIP = 13;
const starStripCache = new Map<number, StripStar[]>();

function getStarStrip(strip: number): StripStar[] {
  let cached = starStripCache.get(strip);
  if (cached) return cached;
  cached = [];
  const stripBig = BigInt.asUintN(64, BigInt(strip));
  for (let index = 0; index < STARS_PER_STRIP; index++) {
    const key = (stripBig * BigInt(STARS_PER_STRIP) + BigInt(index)) & MASK64;
    let bits = (key + 0x9e3779b97f4a7c15n) & MASK64;
    bits = ((bits ^ (bits >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK64;
    bits = ((bits ^ (bits >> 27n)) * 0x94d049bb133111ebn) & MASK64;
    bits = (bits ^ (bits >> 31n)) & MASK64;
    const frac = (shift: bigint) => Number((bits >> shift) & 0xffffn) / 65536;
    cached.push({
      offset: frac(0n) * STRIP_WIDTH,
      y: frac(16n) * 270,
      size: bits >> 60n <= 1n ? 2.4 : 1.2,
      phase: frac(32n) * TAU,
    });
  }
  starStripCache.set(strip, cached);
  return cached;
}

export function drawSky(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  time: number,
  scale: number,
  starCamera: number,
): void {
  ctx.fillStyle = rgbCss(0x41324b);
  for (let band = 0; band < 10; band++) {
    ctx.globalAlpha = 0.025 + band * 0.013;
    ctx.fillRect(0, (h * 0.48 + band * h * 0.052) * scale, w * scale, h * 0.055 * scale);
  }
  const shift = starCamera * 0.06;
  const first = Math.floor((shift - 2.4) / STRIP_WIDTH);
  const last = Math.floor((shift + w) / STRIP_WIDTH);
  ctx.fillStyle = rgbCss(0xc2c3e1);
  for (let strip = first; strip <= last; strip++) {
    const base = strip * STRIP_WIDTH - shift;
    const stars = getStarStrip(strip);
    for (let i = 0; i < stars.length; i++) {
      const s = stars[i]!;
      const x = base + s.offset;
      if (x + s.size <= 0 || x >= w) continue;
      ctx.globalAlpha = 0.22 + 0.35 * (Math.sin(time * 0.7 + s.phase) * 0.5 + 0.5);
      const r = s.size * 0.5 * scale;
      ctx.beginPath();
      ctx.arc((x + s.size * 0.5) * scale, (s.y + s.size * 0.5) * scale, r, 0, TAU);
      ctx.fill();
    }
  }
  ctx.globalAlpha = 1;
}

const WINDOW_SPAN = 66;
const WINDOW_TILE_HEIGHT = 108;

let skylineWindowTiles: [CachedTile[], CachedTile[]] | null = null;
function getSkylineWindowTiles(): [CachedTile[], CachedTile[]] {
  if (skylineWindowTiles) return skylineWindowTiles;
  const makeGroup = (glow: number) =>
    Array.from({ length: 4 }, (_, phase) =>
      createTileCanvas(0, 0, WINDOW_SPAN, WINDOW_TILE_HEIGHT, (ctx) => {
        ctx.fillStyle = rgbCss(0xd9b49b);
        for (let row = 0; row < 4; row++) {
          for (let col = 0; col < 4; col++) {
            const value = (phase * 11 + row * 7 + col * (5 + row) + col * col) % 13;
            const brightness = 0.42 + value * 0.052;
            ctx.globalAlpha = glow * brightness;
            ctx.fillRect(col * 20, row * 27, 6, 10);
          }
        }
        ctx.globalAlpha = 1;
      }),
    );
  skylineWindowTiles = [makeGroup(0.13), makeGroup(0.27)];
  return skylineWindowTiles;
}

const levelMoonCache = new Map<number, CachedTile>();
function getCourseMoonTile(level: number): CachedTile {
  const key = Math.min(level, 2);
  let tile = levelMoonCache.get(key);
  if (!tile) {
    const pad = COURSE_MOON_DIAMETER;
    const total = COURSE_MOON_DIAMETER + pad * 2;
    tile = createTileCanvas(
      -pad,
      -pad,
      total,
      total,
      (ctx) => {
        drawArtPaths(ctx, moonPaths(0, 0, COURSE_MOON_DIAMETER, key), 1);
      },
      2,
    );
    levelMoonCache.set(key, tile);
  }
  return tile;
}

export function drawCourseSkyline(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  scale: number,
  camera: number,
): void {
  const windowGroups = getSkylineWindowTiles();

  for (let layer = 0; layer < SKYLINE_ROWS.length; layer++) {
    const row = SKYLINE_ROWS[layer]!;
    const offsetX = -camera * row.parallax;
    const first = Math.floor(-offsetX / row.spacing);
    const end = first + Math.floor(w / row.spacing) + 3;
    const tiles = windowGroups[layer === 0 ? 0 : 1];
    ctx.fillStyle = rgbCss(row.ink);
    ctx.globalAlpha = 1;

    const pendingWindows: [number, number, number][] = [];
    for (let i = first; i < end; i++) {
      const x = i * row.spacing + offsetX;
      if (x > w) break;
      const top = skylineRowTop(row, i, layer);
      if (top >= h) continue;
      const bw = row.spacing - 9;
      ctx.beginPath();
      ctx.roundRect(x * scale, top * scale, bw * scale, (h - top) * scale, [
        3 * scale,
        3 * scale,
        0,
        0,
      ]);
      ctx.fill();
      ctx.fillRect((x + 13) * scale, (top - 9) * scale, (row.spacing - 35) * scale, 10 * scale);
      if (i % 3 === 0) {
        ctx.beginPath();
        ctx.roundRect((x + 35) * scale, (top - 30) * scale, 21 * scale, 24 * scale, [
          7 * scale,
          7 * scale,
          0,
          0,
        ]);
        ctx.fill();
        ctx.fillRect((x + 41) * scale, (top - 46) * scale, 2 * scale, 17 * scale);
      }
      if (top + 17 < h) {
        pendingWindows.push([x + (bw - WINDOW_SPAN) * 0.5, top + 17, remEuclid(i, 4)]);
      }
    }

    for (const [wx, wy, phase] of pendingWindows) {
      const tile = tiles[phase]!;
      ctx.save();
      ctx.beginPath();
      ctx.rect(wx * scale, wy * scale, WINDOW_SPAN * scale, (h - wy) * scale);
      ctx.clip();
      const rows = Math.ceil((h - wy) / WINDOW_TILE_HEIGHT);
      for (let r = 0; r < rows; r++) {
        ctx.drawImage(
          tile.canvas,
          wx * scale,
          (wy + r * WINDOW_TILE_HEIGHT) * scale,
          WINDOW_SPAN * scale,
          WINDOW_TILE_HEIGHT * scale,
        );
      }
      ctx.restore();
    }
  }
}

function drawCourseMoon(
  ctx: CanvasRenderingContext2D,
  w: number,
  scale: number,
  level: number,
  verticalOffset: number,
): void {
  const moonX = w - 155;
  const screenMoonY = (58 + verticalOffset) * scale;
  const minScreenY = scale < 0.9 || w < 620 ? 74 : 58 * scale;
  const moonY = screenMoonY < minScreenY ? minScreenY / scale - verticalOffset : 58;
  paintTile(ctx, getCourseMoonTile(level), moonX, moonY, scale);
}

/** Build a single union Path2D (nonzero winding) for the cat body so opacity never creates seams. */
function buildCatPath(pose: Pose): Path2D {
  const path = new Path2D();
  const addRound = (x: number, y: number, w: number, h: number, radius: number) => {
    if (w <= 0 || h <= 0) return;
    const r = Math.min(radius, w * 0.5, h * 0.5);
    path.roundRect(x, y, w, h, r);
  };
  if (pose.home) {
    addRound(-11, -14, 37, 14, 7);
    addRound(13, -19, 16, 16, 8);
    addRound(20, -20, 4, 7, 0);
  } else {
    const x = -20;
    const compression = pose.squash * 5;
    const y = -36 + compression;
    const head = pose.facing > 0 ? 21 : 0;
    const body = pose.facing > 0 ? 3 : 12;
    const near = head + (pose.facing > 0 ? 2 : 13);
    const far = head + (pose.facing > 0 ? 5 : 11);
    addRound(x + body, y + 15, 27 + compression, 18 - compression, 9);
    addRound(x + head, y + 7, 19, 20, 7);
    addRound(x + far, y + 4, 3, 8, 1.5);
    addRound(x + near, y + 1, 4, 11, 2);
    addRound(x + body + 4, y + 29 - compression, 6, 6 + pose.gait, 3);
    addRound(x + body + 19, y + 29 - compression, 6, 6 - pose.gait, 3);
    const tail = pose.facing > 0 ? body + 2 : body + 21;
    for (let i = 0; i < 5; i++) {
      addRound(
        x + tail - pose.facing * i * 3,
        y + 22 - i * 2.8 + (i === 0 ? 0 : pose.gait * 0.25),
        7,
        7,
        3.5,
      );
    }
  }
  return path;
}

class CatArtCache {
  private readonly cache = new Map<string, Path2D>();

  pathFor(pose: Pose): Path2D {
    const right = pose.facing > 0 ? 1 : 0;
    const home = pose.home ? 1 : 0;
    const gaitQ = Math.round(clamp(pose.gait, -3, 3) * 4);
    const squashQ = Math.round(clamp(pose.squash, 0, 1) * 12);
    const key = `${right}:${home}:${gaitQ}:${squashQ}`;
    let path = this.cache.get(key);
    if (!path) {
      path = buildCatPath({
        ...pose,
        facing: right ? 1 : -1,
        gait: gaitQ / 4,
        squash: squashQ / 12,
      });
      this.cache.set(key, path);
    }
    return path;
  }
}

function drawCatBody(
  ctx: CanvasRenderingContext2D,
  catArt: CatArtCache,
  pose: Pose,
  camera: number,
  scale: number,
  ink: number,
  opacity: number,
  spriteScale = 1,
): void {
  const path = catArt.pathFor(pose);
  ctx.save();
  ctx.translate((pose.x - camera) * scale, pose.y * scale);
  ctx.scale(scale * spriteScale, scale * spriteScale);
  ctx.globalAlpha = opacity;
  ctx.fillStyle = rgbCss(ink);
  ctx.fill(path, "nonzero");
  ctx.restore();
  ctx.globalAlpha = 1;
}

function drawPlayerSprite(
  ctx: CanvasRenderingContext2D,
  catArt: CatArtCache,
  pose: Pose,
  camera: number,
  opacity: number,
  scale: number,
  spriteScale = 1,
): void {
  drawCatBody(ctx, catArt, pose, camera, scale, 0x05070d, opacity, spriteScale);
  const head = pose.facing > 0 ? 21 : 0;
  const near = head + (pose.facing > 0 ? 2 : 13);
  const x = -20;
  const y = -36 + pose.squash * 5;
  ctx.save();
  ctx.translate((pose.x - camera) * scale, pose.y * scale);
  ctx.scale(scale * spriteScale, scale * spriteScale);
  ctx.globalAlpha = opacity;
  ctx.fillStyle = rgbCss(0x71506e);
  ctx.fillRect(x + near + 1, y + 6, 2, 3);
  ctx.fillStyle = rgbCss(0xa8f0ce);
  ctx.beginPath();
  ctx.roundRect(x + head + (pose.facing > 0 ? 10 : 3), y + 15, 5, 3, 1.5);
  ctx.fill();
  ctx.restore();
  ctx.globalAlpha = 1;
}

function drawJetpack(
  ctx: CanvasRenderingContext2D,
  bottomCenterX: number,
  bottomY: number,
  flame: number,
  fuel: number,
  scale: number,
  spriteScale = 1,
): void {
  ctx.save();
  ctx.translate(bottomCenterX * scale, bottomY * scale);
  ctx.scale(scale * spriteScale, scale * spriteScale);
  ctx.globalAlpha = 1;
  // Local coordinates place nozzle bottoms at y = 0 and twin cylinders centered around x = 0.
  const x = -8;
  const y = -22;
  for (const offset of [0, 9]) {
    ctx.fillStyle = rgbCss(0x9e91b5);
    ctx.beginPath();
    ctx.roundRect(x + offset, y, 7, 20, 3);
    ctx.fill();

    ctx.fillStyle = rgbCss(0xd5c7e6);
    ctx.beginPath();
    ctx.roundRect(x + offset + 1, y + 2, 3, 13, 1.5);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x73d9ee);
    ctx.fillRect(x + offset, y + 8, 7, 3);

    ctx.fillStyle = rgbCss(0x50465f);
    ctx.fillRect(x + offset + 1, y + 18, 5, 4);

    if (flame > 0) {
      ctx.fillStyle = rgbCss(0x74cfea);
      ctx.beginPath();
      ctx.roundRect(x + offset, y + 21, 7, flame, [0, 0, 4, 4]);
      ctx.fill();

      ctx.fillStyle = rgbCss(GOLD);
      ctx.beginPath();
      ctx.roundRect(x + offset + 2, y + 21, 3, flame * 0.65, [0, 0, 2, 2]);
      ctx.fill();
    }
  }
  ctx.fillStyle = rgbCss(0x50465f);
  ctx.fillRect(x - 1, y - 7, 18, 4);
  if (fuel > 0) {
    ctx.fillStyle = rgbCss(GOLD);
    ctx.fillRect(x, y - 6, 16 * fuel, 2);
  } else {
    ctx.fillStyle = rgbCss(0xff6b81);
    ctx.fillRect(x, y - 6, 2, 2);
  }
  ctx.restore();
}

function drawArrivalSequence(
  ctx: CanvasRenderingContext2D,
  catArt: CatArtCache,
  time: number,
  camera: number,
  scale: number,
  spriteScale = 1,
): void {
  const flight = Math.min(time / ARRIVAL_FLAME_OUT, 1);
  const fall = clamp((time - ARRIVAL_FLAME_OUT) / (ARRIVAL_LANDING - ARRIVAL_FLAME_OUT), 0, 1);
  const x = START_X - 150 * (1 - flight) * (1 - flight) - camera;
  const y = ROOFS[0]!.y - 96 * (1 - fall * fall) - 85 * (1 - flight);
  const squash = clamp(1 - (time - ARRIVAL_LANDING) / 0.2, 0, 1);
  drawPlayerSprite(
    ctx,
    catArt,
    {
      x: x + camera,
      y,
      facing: 1,
      gait: 0,
      squash: time < ARRIVAL_LANDING ? 0 : squash,
      home: false,
    },
    camera,
    1,
    scale,
    spriteScale,
  );
  const flame = arrivalFlamesOn(time) ? 22 * (1 - flight) + 4 : 0;
  const detach = clamp((time - ARRIVAL_UNSTRAP) / (ARRIVAL_DURATION - ARRIVAL_UNSTRAP), 0, 1);
  // Start strapped to the scaled cat's back and settle flush onto roof 0 at START_X - 44.
  const attachedX = x - 6 * spriteScale;
  const attachedY = y - 13 * spriteScale;
  const parkedX = x - 44;
  const parkedY = y;
  drawJetpack(
    ctx,
    attachedX + (parkedX - attachedX) * detach,
    attachedY + (parkedY - attachedY) * detach,
    flame,
    1 - flight,
    scale,
    spriteScale,
  );
}

function drawEnemy(
  ctx: CanvasRenderingContext2D,
  enemy: Enemy,
  cx: number,
  cy: number,
  time: number,
  scale: number,
  spriteScale = 1,
): void {
  if (spriteScale !== 1) {
    // Anchor rats to the roof surface (cy + 11) so larger mobile sprites never sink into roofs;
    // scale airborne owls and bats around their center.
    const pivotY = enemy.kind === "rat" ? cy + 11 : cy;
    ctx.save();
    ctx.translate(cx * scale, pivotY * scale);
    ctx.scale(spriteScale, spriteScale);
    ctx.translate(-cx * scale, -pivotY * scale);
    drawEnemy(ctx, enemy, cx, cy, time, scale, 1);
    ctx.restore();
    return;
  }
  const ox = cx - 17;
  const oy = cy - 11;
  const right = Math.cos(time * enemy.speed + enemy.phase) > 0;
  ctx.globalAlpha = 1;

  if (enemy.kind === "rat") {
    const nose = right ? 26 : 1;
    ctx.fillStyle = rgbCss(0x72516b);
    ctx.beginPath();
    ctx.roundRect((ox + 4) * scale, (oy + 7) * scale, 26 * scale, 16 * scale, 8 * scale);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x966882);
    ctx.beginPath();
    ctx.arc((ox + (right ? 25 : 10)) * scale, (oy + 6) * scale, 5 * scale, 0, TAU);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x9c708a);
    ctx.beginPath();
    ctx.arc((ox + nose + 3.5) * scale, (oy + 13.5) * scale, 3.5 * scale, 0, TAU);
    ctx.fill();

    ctx.fillStyle = rgbCss(0xffa27d);
    ctx.fillRect((ox + (right ? 23 : 7)) * scale, (oy + 10) * scale, 3 * scale, 3 * scale);

    ctx.fillStyle = rgbCss(0xa0758d);
    ctx.beginPath();
    ctx.roundRect(
      (ox + (right ? -7 : 29)) * scale,
      (oy + 15) * scale,
      13 * scale,
      3 * scale,
      1.5 * scale,
    );
    ctx.fill();
  } else if (enemy.kind === "owl") {
    const wing = Math.sin(time * 9 + enemy.phase) * 7;
    ctx.fillStyle = rgbCss(0x746080);
    ctx.beginPath();
    ctx.roundRect((ox - 8) * scale, (oy + 8 + wing) * scale, 20 * scale, 8 * scale, 4 * scale);
    ctx.roundRect((ox + 23) * scale, (oy + 8 + wing) * scale, 20 * scale, 8 * scale, 4 * scale);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x9b7e9e);
    ctx.beginPath();
    ctx.roundRect((ox + 8) * scale, (oy + 2) * scale, 20 * scale, 23 * scale, 9 * scale);
    ctx.fill();

    ctx.fillStyle = rgbCss(0xf4d4a8);
    ctx.beginPath();
    ctx.arc((ox + 13.5) * scale, (oy + 9.5) * scale, 3.5 * scale, 0, TAU);
    ctx.arc((ox + 23.5) * scale, (oy + 9.5) * scale, 3.5 * scale, 0, TAU);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x292031);
    ctx.fillRect((ox + 13) * scale, (oy + 8) * scale, 2 * scale, 3 * scale);
    ctx.fillRect((ox + 22) * scale, (oy + 8) * scale, 2 * scale, 3 * scale);

    ctx.fillStyle = rgbCss(0xe6a47c);
    ctx.fillRect((ox + 17) * scale, (oy + 14) * scale, 3 * scale, 4 * scale);
  } else {
    const flap = Math.sin(time * 12 + enemy.phase) * 5;
    const batWing: ArtPath = {
      points: [
        [ox + 18, oy + 10],
        [ox + 9, oy + 6],
        [ox - 9, oy + 5 + flap],
        [ox - 3, oy + 18 + flap],
        [ox + 3, oy + 14],
        [ox + 10, oy + 20],
        [ox + 18, oy + 16],
        [ox + 25, oy + 20],
        [ox + 32, oy + 14],
        [ox + 39, oy + 18 + flap],
        [ox + 45, oy + 5 + flap],
        [ox + 27, oy + 6],
      ],
      ink: 0x81648c,
      opacity: 1,
    };
    drawArtPaths(ctx, [batWing], scale);

    ctx.fillStyle = rgbCss(0x513d61);
    ctx.beginPath();
    ctx.arc((ox + 18) * scale, (oy + 13) * scale, 7 * scale, 0, TAU);
    ctx.fill();

    ctx.fillStyle = rgbCss(0x695075);
    ctx.beginPath();
    ctx.roundRect((ox + 12) * scale, (oy + 2) * scale, 4 * scale, 9 * scale, [
      2 * scale,
      2 * scale,
      0,
      0,
    ]);
    ctx.roundRect((ox + 20) * scale, (oy + 2) * scale, 4 * scale, 9 * scale, [
      2 * scale,
      2 * scale,
      0,
      0,
    ]);
    ctx.fill();

    ctx.fillStyle = rgbCss(0xffc397);
    ctx.fillRect((ox + 14) * scale, (oy + 10) * scale, 2 * scale, 2 * scale);
    ctx.fillRect((ox + 20) * scale, (oy + 10) * scale, 2 * scale, 2 * scale);
  }
}

function drawShelter(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  home: boolean,
  lit: boolean,
  palette: CheckpointPalette,
  scale: number,
): void {
  if (home) {
    drawArtPaths(ctx, homeRoofPaths(x, y, 1, lit), scale);
  }
  const shapes = home ? shelterShapes(x, y, true, lit, 1) : checkpointShapes(x, y, lit, 1, palette);
  for (const shape of shapes) {
    drawPosterShape(ctx, shape, scale);
  }
  ctx.globalAlpha = 1;
}

const statIconTileCache = new Map<StatIcon, CachedTile>();
function getStatIconTile(kind: StatIcon): CachedTile {
  let tile = statIconTileCache.get(kind);
  if (!tile) {
    tile = createTileCanvas(
      0,
      0,
      20,
      20,
      (ctx) => {
        drawArtPaths(ctx, statPaths(kind, 0, 0, 20, SKY), 1);
      },
      2,
    );
    statIconTileCache.set(kind, tile);
  }
  return tile;
}

function drawControlPill(
  ctx: CanvasRenderingContext2D,
  label: string,
  centerX: number,
  bottomY: number,
): void {
  ctx.font = `400 14px ${POSTER_FONT}`;
  const textWidth = ctx.measureText(label).width;
  const padX = 16;
  const height = 36;
  const width = textWidth + padX * 2;
  const x = centerX - width * 0.5;
  const y = bottomY - height;
  const radius = height * 0.5;

  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(SKY);
  ctx.strokeStyle = rgbCss(0x4c4256);
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.roundRect(x, y, width, height, radius);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = rgbCss(0xf3ddba);
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(label, centerX, y + height * 0.5);
  ctx.textAlign = "left";
}

export interface CourseViewport {
  scale: number;
  viewW: number;
  viewH: number;
  verticalOffset: number;
  cameraLead: number;
}

/**
 * Compute course scale and vertical framing.
 * Desktop keeps the exact native 900x560 layout; mobile portrait and short
 * mobile landscape zoom in so the cat, jumps, and rooftops fill the screen
 * cleanly without wasted vertical space.
 */
export function computeCourseViewport(viewportW: number, viewportH: number): CourseViewport {
  const portrait = viewportH > viewportW;
  if (portrait && viewportW < 760) {
    const targetW = clamp(viewportW * 1.18, 430, 560);
    const scale = Math.max(0.1, viewportW / targetW);
    const viewW = viewportW / scale;
    const viewH = viewportH / scale;
    const verticalOffset = Math.max(0, (viewH - 560) * 0.42);
    return { scale, viewW, viewH, verticalOffset, cameraLead: 0.27 };
  }
  if (!portrait && viewportH < 500) {
    const scale = Math.max(0.1, Math.min(viewportW / 620, viewportH / 445));
    const viewW = viewportW / scale;
    const viewH = viewportH / scale;
    const verticalOffset = clamp((viewH - 540) * 0.55, -62, 0);
    return { scale, viewW, viewH, verticalOffset, cameraLead: 0.31 };
  }
  const scale = Math.max(0.1, Math.min(viewportW / 900, viewportH / 560));
  return {
    scale,
    viewW: viewportW / scale,
    viewH: viewportH / scale,
    verticalOffset: 0,
    cameraLead: 0.34,
  };
}

export interface TouchControlsLayout {
  joyBaseX: number;
  joyBaseY: number;
  joyRadius: number;
  knobRadius: number;
  jumpX: number;
  jumpY: number;
  jumpRadius: number;
}

export function computeTouchControlsLayout(
  viewportW: number,
  viewportH: number,
  safeBottom = 0,
  safeLeft = 0,
  safeRight = 0,
): TouchControlsLayout {
  const shortSide = Math.min(viewportW, viewportH);
  const radius = clamp(shortSide * 0.13, 42, 58);
  const knobRadius = radius * 0.43;
  const edgeX = clamp(viewportW * 0.045, 18, 34);
  const edgeY = clamp(viewportH * 0.045, 16, 32);
  const centerY = viewportH - safeBottom - edgeY - radius;
  return {
    joyBaseX: safeLeft + edgeX + radius,
    joyBaseY: centerY,
    joyRadius: radius,
    knobRadius,
    jumpX: viewportW - safeRight - edgeX - radius,
    jumpY: centerY,
    jumpRadius: radius,
  };
}

export interface TouchControlState {
  layout: TouchControlsLayout;
  joyActive: boolean;
  joyOriginX: number;
  joyOriginY: number;
  knobX: number;
  knobY: number;
  direction: -1 | 0 | 1;
  jumpPressed: boolean;
  doorwayReady: boolean;
}

export interface CourseHudParams {
  viewportW: number;
  viewportH: number;
  score: Score;
  level: number;
  nameFirst: boolean;
  clockOpacity: number;
  gap: GhostGap | null;
  showArrivalCaption: boolean;
  showDoorwayPrompt: boolean;
  touchMode: boolean;
  touchControls: TouchControlState | null;
  safeTop: number;
  safeLeft: number;
  safeRight: number;
  safeBottom: number;
}

function drawSubtleHintPill(
  ctx: CanvasRenderingContext2D,
  label: string,
  centerX: number,
  bottomY: number,
): void {
  ctx.font = `500 12px ${POSTER_FONT}`;
  const textWidth = ctx.measureText(label).width;
  const padX = 12;
  const height = 24;
  const width = textWidth + padX * 2;
  const x = centerX - width * 0.5;
  const y = bottomY - height;

  ctx.save();
  ctx.fillStyle = "rgb(16 19 34 / 0.84)";
  ctx.strokeStyle = "rgb(76 66 86 / 0.7)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.roundRect(x, y, width, height, height * 0.5);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = rgbCss(0xe6ddf0);
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(label, centerX, y + height * 0.5);
  ctx.restore();
}

function drawTouchControls(ctx: CanvasRenderingContext2D, state: TouchControlState): void {
  const {
    layout,
    joyActive,
    joyOriginX,
    joyOriginY,
    knobX,
    knobY,
    direction,
    jumpPressed,
    doorwayReady,
  } = state;
  const { joyRadius, knobRadius, jumpX, jumpY, jumpRadius } = layout;

  // Bottom-left translucent virtual joystick: whisper-light at rest, clearer while held.
  ctx.save();
  ctx.fillStyle = joyActive ? "rgb(16 19 34 / 0.42)" : "rgb(16 19 34 / 0.11)";
  ctx.strokeStyle = joyActive ? "rgb(255 219 160 / 0.65)" : "rgb(243 221 186 / 0.16)";
  ctx.lineWidth = joyActive ? 2 : 1.5;
  ctx.beginPath();
  ctx.arc(joyOriginX, joyOriginY, joyRadius, 0, TAU);
  ctx.fill();
  ctx.stroke();

  // Subtle left/right directional chevrons inside the stick ring.
  const drawChevron = (cx: number, cy: number, dir: -1 | 1, active: boolean) => {
    ctx.strokeStyle = active
      ? "rgb(255 219 160 / 0.92)"
      : joyActive
        ? "rgb(243 221 186 / 0.42)"
        : "rgb(243 221 186 / 0.18)";
    ctx.lineWidth = 2;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(cx - dir * 3, cy - 6);
    ctx.lineTo(cx + dir * 3.5, cy);
    ctx.lineTo(cx - dir * 3, cy + 6);
    ctx.stroke();
  };
  drawChevron(joyOriginX - joyRadius * 0.68, joyOriginY, -1, direction === -1);
  drawChevron(joyOriginX + joyRadius * 0.68, joyOriginY, 1, direction === 1);

  // Movable thumb knob.
  ctx.fillStyle =
    direction !== 0
      ? "rgb(255 219 160 / 0.62)"
      : joyActive
        ? "rgb(243 221 186 / 0.50)"
        : "rgb(243 221 186 / 0.14)";
  ctx.strokeStyle = joyActive ? "rgb(255 237 201 / 0.85)" : "rgb(243 221 186 / 0.24)";
  ctx.lineWidth = joyActive ? 2 : 1.5;
  ctx.beginPath();
  ctx.arc(knobX, knobY, knobRadius, 0, TAU);
  ctx.fill();
  ctx.stroke();

  // Bottom-right translucent Jump button: whisper-light at rest, clearly lit while held.
  ctx.fillStyle = jumpPressed
    ? "rgb(255 219 160 / 0.44)"
    : doorwayReady
      ? "rgb(255 219 160 / 0.18)"
      : "rgb(16 19 34 / 0.11)";
  ctx.strokeStyle = jumpPressed
    ? "rgb(255 219 160 / 0.88)"
    : doorwayReady
      ? "rgb(255 219 160 / 0.48)"
      : "rgb(243 221 186 / 0.16)";
  ctx.lineWidth = jumpPressed ? 2 : 1.5;
  ctx.beginPath();
  ctx.arc(jumpX, jumpY, jumpRadius, 0, TAU);
  ctx.fill();
  ctx.stroke();

  // Crisp upward jump arrow icon centered inside the button.
  const arrowSize = jumpRadius * 0.34;
  ctx.strokeStyle = jumpPressed
    ? "rgb(255 245 224 / 0.96)"
    : doorwayReady
      ? "rgb(255 219 160 / 0.62)"
      : "rgb(255 219 160 / 0.32)";
  ctx.lineWidth = jumpPressed ? 3 : 2.4;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(jumpX, jumpY + arrowSize * 0.85);
  ctx.lineTo(jumpX, jumpY - arrowSize * 0.85);
  ctx.moveTo(jumpX - arrowSize * 0.78, jumpY - arrowSize * 0.12);
  ctx.lineTo(jumpX, jumpY - arrowSize * 0.9);
  ctx.lineTo(jumpX + arrowSize * 0.78, jumpY - arrowSize * 0.12);
  ctx.stroke();
  ctx.restore();
}

/** Draw all non-interactive gameplay HUD elements directly onto Canvas 2D. */
export function drawCourseHud(ctx: CanvasRenderingContext2D, hud: CourseHudParams): void {
  const {
    viewportW,
    viewportH,
    score,
    level,
    nameFirst,
    clockOpacity,
    gap,
    showArrivalCaption,
    showDoorwayPrompt,
    touchMode,
    touchControls,
    safeTop,
    safeLeft,
    safeRight,
    safeBottom,
  } = hud;
  const narrow = viewportW < 560;
  const shortScreen = viewportH < 500;
  const compactMobile = touchMode || narrow || shortScreen;

  // Top-left purple stat counters, vertically aligned with the top-right Options button.
  const iconScale = compactMobile ? 1.05 : 1;
  const iconSize = 20 * iconScale;
  const barCenterY = compactMobile ? Math.max(32, safeTop + 30) : 29;
  const statY = barCenterY - iconSize * 0.5;
  ctx.globalAlpha = 1;
  ctx.font = `600 ${compactMobile ? 14 : 12}px ${POSTER_FONT}`;
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  ctx.fillStyle = rgbCss(PURPLE);
  let cursorX = Math.max(20, safeLeft + 16);
  const statGap = narrow ? 14 : 18;
  const entries: readonly [StatIcon, number][] = [
    ["coin", score.coins],
    ["deaths", score.deaths],
    ["kills", score.kills],
  ];
  for (const [kind, count] of entries) {
    paintTile(ctx, getStatIconTile(kind), cursorX, statY, iconScale);
    const label = String(count);
    ctx.fillText(label, cursorX + iconSize + 6, barCenterY);
    cursorX += iconSize + 6 + ctx.measureText(label).width + statGap;
  }

  // Top-center neighborhood / clock stack: stays compact in the top sky band on mobile
  // so it never overlaps the cat or high rooftop jump arcs.
  if (clockOpacity > 0.001) {
    const name = neighborhoodTitle(neighborhoodForLevel(level));
    const time = timeLabel(score.seconds);
    const primary = nameFirst ? name : time;
    const secondary = nameFirst ? time : name;
    const primaryColor = nameFirst ? GOLD : PURPLE;
    const secondaryColor = nameFirst ? PURPLE : GOLD;

    let primarySize: number;
    let primaryLine: number;
    let secondarySize: number;
    let topOffset: number;
    if (shortScreen) {
      primarySize = nameFirst ? 22 : 28;
      primaryLine = nameFirst ? 26 : 32;
      secondarySize = 12;
      topOffset = Math.max(10, safeTop + 8);
    } else if (narrow) {
      primarySize = nameFirst ? 28 : 40;
      primaryLine = nameFirst ? 34 : 46;
      secondarySize = 14;
      topOffset = Math.max(58, safeTop + 54);
    } else {
      primarySize = nameFirst ? 42 : 70;
      primaryLine = nameFirst ? 52 : 80;
      secondarySize = 20;
      topOffset = 16;
    }

    ctx.globalAlpha = clockOpacity;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.font = `600 ${primarySize}px ${POSTER_FONT}`;
    ctx.fillStyle = rgbCss(primaryColor);
    ctx.fillText(primary, viewportW * 0.5, topOffset + primaryLine * 0.5);

    ctx.font = `600 ${secondarySize}px ${POSTER_FONT}`;
    ctx.fillStyle = rgbCss(secondaryColor);
    ctx.fillText(secondary, viewportW * 0.5, topOffset + primaryLine + secondarySize * 0.65);
    ctx.globalAlpha = 1;
  }

  // Offscreen ghost gap indicator.
  if (gap !== null) {
    const y = viewportH * 0.43;
    ctx.fillStyle = rgbCss(gap.ahead ? PLAYER_AHEAD_GREEN : PLAYER_BEHIND_RED);
    ctx.textBaseline = "top";
    if (gap.ahead) {
      const lx = Math.max(20, safeLeft + 16);
      ctx.textAlign = "left";
      ctx.font = `600 18px ${POSTER_FONT}`;
      ctx.fillText(`← ${gap.meters.toFixed(1)} m`, lx, y);
      ctx.font = `600 10px ${POSTER_FONT}`;
      ctx.fillText("AHEAD", lx, y + 24);
    } else {
      const rx = viewportW - Math.max(20, safeRight + 16);
      ctx.textAlign = "right";
      ctx.font = `600 18px ${POSTER_FONT}`;
      ctx.fillText(`${gap.meters.toFixed(1)} m →`, rx, y);
      ctx.font = `600 10px ${POSTER_FONT}`;
      ctx.fillText("BEHIND", rx, y + 24);
    }
    ctx.textAlign = "left";
  }

  // Arrival opening caption (on touch devices the translucent joystick + Jump button are self-explanatory).
  if (showArrivalCaption) {
    const baseBottom = viewportH - Math.max(22, safeBottom + 16);
    if (touchMode) {
      drawControlPill(ctx, "Out of fuel and far from home.", viewportW * 0.5, baseBottom);
    } else {
      drawSubtleHintPill(ctx, "← → move · Space jump", viewportW * 0.5, baseBottom);
      drawControlPill(ctx, "Out of fuel and far from home.", viewportW * 0.5, baseBottom - 30);
    }
  }

  // Checkpoint doorway prompt.
  if (showDoorwayPrompt) {
    drawControlPill(
      ctx,
      touchMode ? "Tap jump for next level" : "Space for next level",
      viewportW * 0.5,
      viewportH - Math.max(20, safeBottom + 16),
    );
  }

  if (touchControls) {
    drawTouchControls(ctx, touchControls);
  }

  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.globalAlpha = 1;
}

export class CourseRenderer {
  private readonly facadeArt = new FacadeArt();
  private readonly roomArt = new RoomArt();
  private readonly catArt = new CatArtCache();
  private coinCanvas: HTMLCanvasElement | null = null;

  constructor() {
    this.loadCoinSprite();
  }

  private loadCoinSprite(): void {
    if (typeof Image === "undefined") return;
    const img = new Image();
    img.decoding = "async";
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = 112;
      canvas.height = 112;
      const ctx = canvas.getContext("2d");
      if (ctx) {
        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = "high";
        ctx.drawImage(img, 0, 0, 112, 112);
        this.coinCanvas = canvas;
      }
    };
    img.src = kittenCoinUrl;
  }

  render(
    ctx: CanvasRenderingContext2D,
    params: {
      run: Run;
      slide: LevelSlide | null;
      arrival: number | null;
      camera: number;
      w: number;
      h: number;
      verticalOffset: number;
      scale: number;
      drift: number;
      reducedMotion: boolean;
      ghost: Frame | null;
      showGhost: boolean;
      touchMode?: boolean;
    },
  ): void {
    const {
      run,
      slide,
      arrival,
      camera,
      w,
      h,
      verticalOffset,
      scale,
      drift,
      reducedMotion,
      ghost,
      showGhost,
      touchMode = false,
    } = params;
    const worldH = h - verticalOffset;
    this.facadeArt.resize(Math.max(h, worldH));

    ctx.fillStyle = rgbCss(SKY);
    ctx.fillRect(0, 0, w * scale, h * scale);

    const backdropCamera = camera + courseProgress(run.level, START_X);
    drawSky(ctx, w, h, drift, scale, backdropCamera);

    ctx.save();
    if (verticalOffset !== 0) {
      ctx.translate(0, verticalOffset * scale);
    }

    drawCourseSkyline(ctx, w, worldH, scale, backdropCamera);
    drawCourseMoon(ctx, w, scale, run.level, verticalOffset);

    if (slide) {
      this.drawCourseBuildings(
        ctx,
        slide.previous,
        camera - slide.offset(),
        w,
        worldH,
        drift,
        scale,
        true,
      );
    }
    this.drawCourseBuildings(ctx, run, camera, w, worldH, drift, scale, false);

    const slideSleeping = slide !== null && slide.sleeping();
    const sleeping = run.home || slideSleeping;
    const lastRoof = run.roofs[run.roofs.length - 1]!;
    const destX = run.destination() - camera;
    if (destX >= -160 && destX <= w + 160) {
      const lastSeed = run.roofIdentity(run.roofs.length - 1)[1];
      drawShelter(
        ctx,
        destX,
        lastRoof.y,
        run.level + 1 === LEVELS,
        !run.home,
        checkpointPaletteFor(neighborhoodForLevel(run.level), lastSeed),
        scale,
      );
    }
    if (run.level > 0 && START_X - camera >= -110 && START_X - camera <= w + 110) {
      const startSeed = run.roofIdentity(0)[1];
      drawShelter(
        ctx,
        START_X - camera,
        run.roofs[0]!.y,
        false,
        !slideSleeping,
        checkpointPaletteFor(neighborhoodForLevel(run.level - 1), startSeed),
        scale,
      );
    }

    // Draw coins & enemies in front of rooftop architecture and checkpoint buildings.
    const courses: [Run, number, boolean][] = slide
      ? [
          [slide.previous, camera - slide.offset(), true],
          [run, camera, false],
        ]
      : [[run, camera, false]];
    const spriteScale = touchMode ? 1.25 : 1;

    for (const [course, courseCam, outgoing] of courses) {
      for (let i = 0; i < course.coins.length; i++) {
        const coin = course.coins[i]!;
        if (coin.collected) continue;
        if (outgoing && i + 1 === course.coins.length) continue;
        const x = coin.x - courseCam;
        if (x < -25 || x > w + 25) continue;
        this.drawCoin(ctx, x, coin.y, scale, touchMode ? 35 : 28);
      }
      const enemyTime = course.time + (slide ? slide.time : 0);
      for (const enemy of course.enemies) {
        if (enemy.defeated) continue;
        const [ex, ey] = enemyPosition(enemy, course.roofs, enemyTime);
        if (ex - courseCam < -60 || ex - courseCam > w + 60) continue;
        drawEnemy(ctx, enemy, ex - courseCam, ey, drift, scale, spriteScale);
      }
    }

    if (showGhost && arrival === null && slide === null && ghost !== null) {
      drawCatBody(ctx, this.catArt, ghost.pose, camera, scale, 0xb8abff, 0.28, spriteScale);
    }

    const airborne = run.ground === null;
    const gait = reducedMotion
      ? 0
      : airborne
        ? 1.5
        : Math.sin(run.time * 26) * (Math.abs(run.vx) / SPEED) * 2.5;

    if (arrival !== null) {
      drawArrivalSequence(ctx, this.catArt, arrival, camera, scale, spriteScale);
    } else if (sleeping) {
      for (const shape of restingCatShapes(run.x - camera, run.y, 0x05070d, spriteScale)) {
        drawPosterShape(ctx, shape, scale);
      }
      ctx.globalAlpha = 1;
      drawArtPaths(ctx, sleepPaths(run.x - camera, run.y, drift, spriteScale), scale);
    } else {
      // Soft ground shadow.
      ctx.globalAlpha = airborne ? 0.18 : 0.45;
      ctx.fillStyle = rgbCss(0x070a12);
      ctx.beginPath();
      ctx.ellipse(
        (run.x - camera) * scale,
        (run.y - 0.5) * scale,
        17 * scale * spriteScale,
        2.5 * scale * spriteScale,
        0,
        0,
        TAU,
      );
      ctx.fill();
      ctx.globalAlpha = 1;

      const flashOpacity =
        run.invulnerable === 0
          ? 1
          : reducedMotion
            ? 0.65
            : Math.floor(run.time * 12) % 2 === 0
              ? 0.5
              : 1;
      drawPlayerSprite(
        ctx,
        this.catArt,
        {
          x: run.x,
          y: run.y,
          facing: run.facing,
          gait,
          squash: reducedMotion ? 0 : run.squash,
          home: run.home,
        },
        camera,
        flashOpacity,
        scale,
        spriteScale,
      );
    }

    if (run.level === 0 && arrival === null) {
      drawJetpack(ctx, START_X - 44 - camera, ROOFS[0]!.y, 0, 0, scale, spriteScale);
    }
    ctx.restore();
  }

  private drawCoin(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    scale: number,
    diameter = 28,
  ): void {
    const half = diameter * 0.5;
    if (this.coinCanvas) {
      ctx.drawImage(
        this.coinCanvas,
        (x - half) * scale,
        (y - half) * scale,
        diameter * scale,
        diameter * scale,
      );
    } else {
      // Fallback gold disc until PNG finishes decoding.
      ctx.fillStyle = rgbCss(GOLD);
      ctx.beginPath();
      ctx.arc(x * scale, y * scale, half * 0.78 * scale, 0, TAU);
      ctx.fill();
    }
  }

  private drawCourseBuildings(
    ctx: CanvasRenderingContext2D,
    run: Run,
    camera: number,
    w: number,
    h: number,
    drift: number,
    scale: number,
    omitLast: boolean,
  ): void {
    for (let i = 0; i < run.roofs.length; i++) {
      const roof = run.roofs[i]!;
      const x = roof.x - camera;
      if (x + roof.width < -80 || x > w + 80 || (omitLast && i + 1 === run.roofs.length)) {
        continue;
      }
      const [identityLevel, seed] = run.roofIdentity(i);
      const neighborhood = neighborhoodForLevel(identityLevel);
      const isLit = run.lit[i] ?? false;

      const wallTile = this.facadeArt.wallTile(neighborhood, roof, seed, isLit);
      paintTile(ctx, wallTile, x, 0, scale);

      const layout = createApartments(roof.width);
      const rows = apartmentRows(roof.y, h);
      for (const [row, wy] of rows) {
        for (let col = 0; col < layout.columns; col++) {
          const wx = x + apartmentWindowX(layout, col);
          if (wx + 72 < 0 || wx - 3 > w) continue;
          const roomSeed = seed * 7 + row * 3 + col;
          if (isLit) {
            const interior =
              row < 4
                ? this.roomArt.interior(roomSeed, drift)
                : this.roomArt.still(seed * 7 + row * 5 + col);
            paintTile(ctx, interior, wx, wy, scale);
            paintTile(ctx, this.facadeArt.openFrame(neighborhood, roomSeed), wx, wy, scale);
          } else {
            paintTile(ctx, this.facadeArt.closedRoom(neighborhood, roomSeed), wx, wy, scale);
          }
        }
      }

      if (i !== run.roofs.length - 1 && !(i === 0 && run.level > 0)) {
        const [brick, coping] = chimneyColorsFor(neighborhood, seed, isLit);
        ctx.globalAlpha = 1;
        ctx.fillStyle = rgbCss(brick);
        ctx.fillRect((x + 17) * scale, (roof.y - 32) * scale, 23 * scale, 32 * scale);
        ctx.fillStyle = rgbCss(coping);
        ctx.fillRect((x + 14) * scale, (roof.y - 34) * scale, 29 * scale, 5 * scale);
        if (i % 2 === 0) {
          ctx.fillStyle = rgbCss(0xb5a8c1);
          for (let puff = 0; puff < 3; puff++) {
            const rise = remEuclid(drift * 11 + puff * 18, 58);
            const d = 14 + rise * 0.23;
            const r = d * 0.5;
            ctx.globalAlpha = (1 - rise / 58) * 0.075;
            ctx.beginPath();
            ctx.arc(
              (x + 18 + rise * 0.15 + r) * scale,
              (roof.y - 45 - rise + r) * scale,
              r * scale,
              0,
              TAU,
            );
            ctx.fill();
          }
          ctx.globalAlpha = 1;
        }
      }

      if (run.hasLantern(i)) {
        const lamp = x + roof.width - 24;
        ctx.globalAlpha = 1;
        ctx.fillStyle = rgbCss(0x7b697b);
        ctx.fillRect(lamp * scale, (roof.y - 43) * scale, 3 * scale, 43 * scale);
        ctx.fillStyle = rgbCss(isLit ? GOLD : 0x756477);
        ctx.beginPath();
        ctx.roundRect((lamp - 6) * scale, (roof.y - 46) * scale, 15 * scale, 16 * scale, 3 * scale);
        ctx.fill();
        if (isLit) {
          ctx.globalAlpha = 0.055;
          ctx.fillStyle = rgbCss(GOLD);
          ctx.beginPath();
          ctx.arc((lamp - 20 + 21.5) * scale, (roof.y - 60 + 21.5) * scale, 21.5 * scale, 0, TAU);
          ctx.fill();
          ctx.globalAlpha = 1;
        }
      }
    }
  }
}
