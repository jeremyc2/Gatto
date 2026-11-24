import { remEuclid, rgbCss, type Point2 } from "./types";

const PHASES = 16;
const ROOM_VARIANTS = 12;
const TAU = Math.PI * 2;
const INK = 0x39303d;
const DENSITY = 2;
const ROOM_W = 66;
const ROOM_H = 62;

export interface CachedTile {
  canvas: HTMLCanvasElement;
  offsetX: number;
  offsetY: number;
  width: number;
  height: number;
}

export function createTileCanvas(
  offsetX: number,
  offsetY: number,
  width: number,
  height: number,
  draw: (ctx: CanvasRenderingContext2D) => void,
  density = DENSITY,
): CachedTile {
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.ceil(width * density));
  canvas.height = Math.max(1, Math.ceil(height * density));
  const ctx = canvas.getContext("2d");
  if (ctx) {
    ctx.setTransform(density, 0, 0, density, -offsetX * density, -offsetY * density);
    draw(ctx);
  }
  return { canvas, offsetX, offsetY, width, height };
}

export function paintTile(
  ctx: CanvasRenderingContext2D,
  tile: CachedTile,
  x: number,
  y: number,
  scale: number,
): void {
  ctx.drawImage(
    tile.canvas,
    (x + tile.offsetX) * scale,
    (y + tile.offsetY) * scale,
    tile.width * scale,
    tile.height * scale,
  );
}

const ROOM_LIGHTS = [0xd4b58b, 0xc9b69a, 0xdac39b, 0xd1aa8c] as const;

export class RoomArt {
  private readonly cache = new Map<number, CachedTile>();

  interior(seed: number, time: number): CachedTile {
    const variant = remEuclid(seed, ROOM_VARIANTS);
    const phase = remEuclid(Math.round(((time * 1.5 + seed) / TAU) * PHASES), PHASES);
    const key = variant * PHASES + phase;
    let tile = this.cache.get(key);
    if (!tile) {
      tile = renderApartment(variant, phase);
      this.cache.set(key, tile);
    }
    return tile;
  }

  still(seed: number): CachedTile {
    return this.interior(seed, 0);
  }
}

function renderApartment(seed: number, phase: number): CachedTile {
  const angle = (phase / PHASES) * TAU;
  const wave = Math.sin(angle);
  const light = ROOM_LIGHTS[seed % 4]!;
  return createTileCanvas(0, 0, ROOM_W, ROOM_H, (ctx) => {
    fillRect(ctx, 0, 0, 66, 62, light, 1);
    fillRect(ctx, 0, 56, 66, 6, 0x8e746e, 1);
    fillRect(ctx, 0, 0, 66, 14, 0xffe0ac, 0.13);

    switch (seed % 6) {
      case 0: {
        // Vacuuming, with a planted stance and long handle.
        const base = 46 + wave * 2;
        const grip: Point2 = [base - 14, 31];
        const parts: readonly [Point2, Point2, [number, number]][] = [
          [
            [8, 56],
            [base - 12, 56],
            [0.8, 0.8],
          ],
          [
            [base - 12, 56],
            [base - 8, 51],
            [0.8, 0.8],
          ],
          [
            [base - 3, 52],
            [base - 10, 39],
            [5.5, 7],
          ],
          [
            [base - 3, 52],
            [base - 16, 27],
            [1.7, 1.7],
          ],
          [
            [base - 16, 27],
            [base - 12, 27],
            [2.3, 2.3],
          ],
        ];
        for (const [a, b, widths] of parts) drawLimb(ctx, a, b, widths);
        drawResident(ctx, {
          shoulder: [24 + wave * 0.3, 24],
          hip: [22, 36],
          hands: [grip, [20, 37]],
          feet: [
            [18, 56],
            [28, 56],
          ],
          facing: 1,
          seated: false,
        });
        fillRounded(ctx, base - 7, 52, 17, 4, 1.5, INK, 1);
        fillCircle(ctx, base - 5, 54, 3, INK, 1);
        fillCircle(ctx, base + 6, 54, 3, INK, 1);
        break;
      }
      case 1: {
        // Playing piano.
        fillRounded(ctx, 37, 29, 25, 13, 3, INK, 1);
        for (const [x, y, w, h] of [
          [34, 41, 28, 3],
          [39, 44, 2, 13],
          [59, 44, 2, 13],
          [14, 44, 17, 3],
          [17, 47, 2, 10],
        ] as const) {
          fillRect(ctx, x, y, w, h, INK, 1);
        }
        fillRect(ctx, 35, 41, 26, 1.5, light, 1);
        drawResident(ctx, {
          shoulder: [23, 29],
          hip: [22, 43],
          hands: [
            [33, 39 + wave * 0.3],
            [35, 40 - wave * 0.3],
          ],
          feet: [
            [32, 56],
            [28, 56],
          ],
          facing: 1,
          seated: true,
        });
        for (let key = 0; key < 7; key++) {
          fillRect(ctx, 36 + key * 3.5, 41, 1, 1, INK, 1);
        }
        break;
      }
      case 2: {
        // Gardening / watering a plant.
        const hand: Point2 = [31 + wave * 0.35, 33 + wave * 0.25];
        drawResident(ctx, {
          shoulder: [43, 23],
          hip: [44, 36],
          hands: [hand, [34, 35]],
          feet: [
            [40, 56],
            [46, 56],
          ],
          facing: -1,
          seated: false,
        });
        fillRounded(ctx, hand[0] - 7, hand[1], 8, 6, 2, INK, 1);
        drawLimb(ctx, [hand[0] - 6, hand[1] + 3], [20, 37], [2, 1.2]);
        fillRounded(ctx, 8, 48, 14, 9, 3, INK, 1);
        drawLimb(ctx, [15, 50], [15, 31], [1.4, 1.1]);
        for (let leaf = 0; leaf < 4; leaf++) {
          const y = 32 + leaf * 4;
          drawLimb(ctx, [15, y + 3], [leaf % 2 === 0 ? 9 : 21, y], [3.8, 1]);
        }
        for (let drop = 0; drop < 2; drop++) {
          fillCircle(ctx, 20, 39 + remEuclid((angle * 7) / TAU + drop * 4, 7), 1, INK, 1);
        }
        break;
      }
      case 3: {
        // Cooking at a stove with rising steam.
        const hand: Point2 = [30 + wave * 0.8, 33 + Math.cos(angle) * 0.4];
        fillRect(ctx, 7, 43, 26, 14, INK, 1);
        fillRounded(ctx, 11, 37, 15, 7, 3, INK, 1);
        fillRect(ctx, 24, 38, 7, 1.5, INK, 1);
        drawResident(ctx, {
          shoulder: [42, 23],
          hip: [45, 36],
          hands: [hand, [35, 34]],
          feet: [
            [41, 56],
            [48, 56],
          ],
          facing: -1,
          seated: false,
        });
        drawLimb(ctx, hand, [23 + wave * 0.8, 41], [1, 1]);
        fillRect(ctx, 10, 47, 20, 7, light, 0.2);
        fillCircle(ctx, 11, 44, 1.5, light, 1);
        fillCircle(ctx, 16, 44, 1.5, light, 1);
        for (let puff = 0; puff < 3; puff++) {
          const rise = remEuclid((angle * 28) / TAU + puff * 5, 14);
          fillCircle(ctx, 14 + rise * 0.2, 30 - rise, 3, INK, (1 - rise / 14) * 0.25);
        }
        break;
      }
      case 4: {
        // Dancing beside a record player.
        drawResident(ctx, {
          shoulder: [33 - wave * 0.7, 22],
          hip: [33 + wave * 0.8, 36],
          hands: [
            [21, 20 + wave],
            [45, 17 - wave],
          ],
          feet: [
            [27, 55],
            [40, 56],
          ],
          facing: 1,
          seated: false,
        });
        for (const [x, y, w, h] of [
          [3, 44, 15, 4],
          [5, 48, 2, 9],
          [14, 48, 2, 9],
        ] as const) {
          fillRect(ctx, x, y, w, h, INK, 1);
        }
        drawLimb(ctx, [7, 41], [15, 41], [2, 2]);
        drawLimb(ctx, [15, 39], [12, 42], [1, 1]);
        break;
      }
      default: {
        // Stargazing with a telescope.
        drawResident(ctx, {
          shoulder: [29, 30],
          hip: [22, 37],
          hands: [
            [39, 33 + wave * 0.2],
            [36, 37],
          ],
          feet: [
            [17, 56],
            [28, 56],
          ],
          facing: 1,
          seated: false,
        });
        const parts: readonly [Point2, Point2, [number, number]][] = [
          [
            [33, 23],
            [55, 17],
            [5, 7],
          ],
          [
            [44, 22],
            [44, 39],
            [2, 2],
          ],
          [
            [44, 38],
            [36, 56],
            [1.5, 1],
          ],
          [
            [44, 38],
            [53, 56],
            [1.5, 1],
          ],
          [
            [44, 38],
            [44, 57],
            [1.5, 1],
          ],
        ];
        for (const [a, b, widths] of parts) drawLimb(ctx, a, b, widths);
        break;
      }
    }

    fillRect(ctx, 0, 0, 4, 56, INK, 0.14);
    fillRect(ctx, 62, 0, 4, 56, INK, 0.14);
    ctx.globalAlpha = 1;
  });
}

interface ResidentPose {
  shoulder: Point2;
  hip: Point2;
  hands: [Point2, Point2];
  feet: [Point2, Point2];
  facing: number;
  seated: boolean;
}

function drawResident(ctx: CanvasRenderingContext2D, pose: ResidentPose): void {
  const { shoulder, hip, hands, feet, facing, seated } = pose;
  const head: Point2 = [shoulder[0] + facing * 0.6, shoulder[1] - 7];
  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(INK);
  ctx.beginPath();
  for (let i = 0; i < 2; i++) {
    const foot = feet[i]!;
    const pelvis: Point2 = [hip[0] + (i - 0.5) * 2.5, hip[1]];
    const knee = elbow(pelvis, foot, 10, seated ? -facing : facing);
    appendLimb(ctx, pelvis, knee, 4.1, 3);
    appendLimb(ctx, knee, foot, 3, 1.9);
    appendLimb(ctx, foot, [foot[0] + facing * 3, foot[1] + 0.3], 2.2, 2.5);
  }
  appendLimb(ctx, shoulder, hip, 7.5, 5.8);
  appendLimb(ctx, [head[0], head[1] + 2], shoulder, 2.6, 3);
  appendLimb(ctx, [head[0], head[1] - 1], [head[0], head[1] + 1.3], 5.4, 4.6);
  appendLimb(ctx, head, [head[0] + facing * 2, head[1] + 0.7], 2.2, 1.8);
  for (let i = 0; i < 2; i++) {
    const hand = hands[i]!;
    const socket: Point2 = [shoulder[0] + (i - 0.5) * 3, shoulder[1] + 0.6];
    const joint = elbow(socket, hand, 7.5, i === 0 ? -1 : 1);
    appendLimb(ctx, socket, joint, 3.2, 2.3);
    appendLimb(ctx, joint, hand, 2.3, 2.2);
  }
  ctx.fill();
}

function elbow(a: Point2, b: Point2, length: number, bend: number): Point2 {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const distance = Math.max(Math.hypot(dx, dy), 0.01);
  const offset = Math.sqrt(Math.max(0, length * length - distance * distance * 0.25)) * bend;
  return [
    (a[0] + b[0]) * 0.5 - (dy / distance) * offset,
    (a[1] + b[1]) * 0.5 + (dx / distance) * offset,
  ];
}

function drawLimb(
  ctx: CanvasRenderingContext2D,
  a: Point2,
  b: Point2,
  widths: [number, number],
): void {
  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(INK);
  ctx.beginPath();
  appendLimb(ctx, a, b, widths[0], widths[1]);
  ctx.fill();
}

const D = Math.SQRT1_2;
const ARC: readonly Point2[] = [
  [0, 1],
  [D, D],
  [1, 0],
  [D, -D],
  [0, -1],
];

function appendLimb(
  ctx: CanvasRenderingContext2D,
  a: Point2,
  b: Point2,
  startWidth: number,
  endWidth: number,
): void {
  const dx0 = b[0] - a[0];
  const dy0 = b[1] - a[1];
  const length = Math.hypot(dx0, dy0);
  const dx = length > 0 ? dx0 / length : 1;
  const dy = length > 0 ? dy0 / length : 0;

  const rEnd = endWidth * 0.5;
  const rStart = -startWidth * 0.5;

  for (let i = 0; i < ARC.length; i++) {
    const [forward, side] = ARC[i]!;
    const px = b[0] + rEnd * (dx * forward - dy * side);
    const py = b[1] + rEnd * (dy * forward + dx * side);
    if (i === 0) ctx.moveTo(px, py);
    else ctx.lineTo(px, py);
  }
  for (let i = 0; i < ARC.length; i++) {
    const [forward, side] = ARC[i]!;
    const px = a[0] + rStart * (dx * forward - dy * side);
    const py = a[1] + rStart * (dy * forward + dx * side);
    ctx.lineTo(px, py);
  }
  ctx.closePath();
}

function fillRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  ink: number,
  alpha: number,
): void {
  ctx.globalAlpha = alpha;
  ctx.fillStyle = rgbCss(ink);
  ctx.fillRect(x, y, w, h);
}

function fillRounded(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
  ink: number,
  alpha: number,
): void {
  ctx.globalAlpha = alpha;
  ctx.fillStyle = rgbCss(ink);
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, Math.min(radius, w * 0.5, h * 0.5));
  ctx.fill();
}

function fillCircle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  diameter: number,
  ink: number,
  alpha: number,
): void {
  const r = diameter * 0.5;
  ctx.globalAlpha = alpha;
  ctx.fillStyle = rgbCss(ink);
  ctx.beginPath();
  ctx.arc(x + r, y + r, r, 0, TAU);
  ctx.fill();
}
