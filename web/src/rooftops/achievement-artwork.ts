import type { Achievement } from "./achievements";
import { rgbCss, type Point2 } from "./types";

export const MEDAL_WIDTH = 96;
export const MEDAL_HEIGHT = 80;
const CENTER: Point2 = [48, 37];
const LINE = 2.2;
const TAU = Math.PI * 2;

interface MedalPalette {
  rim: readonly [number, number, number];
  face: readonly [number, number, number];
  ink: number;
  accent: number;
}

const SILVER: MedalPalette = {
  rim: [0x8f9bb9, 0x4c5773, 0x76819e],
  face: [0x343d55, 0x272e44, 0x1d2437],
  ink: 0xadbad8,
  accent: 0x8898bd,
};

const GOLD: MedalPalette = {
  rim: [0xffedba, 0xad663d, 0xf4bf76],
  face: [0x5a4253, 0x392c43, 0x27213a],
  ink: 0xffeac2,
  accent: 0xf8bb76,
};

export function drawMedalToCanvas(
  canvas: HTMLCanvasElement,
  award: Achievement,
  unlocked: boolean,
): void {
  const density = 2;
  canvas.width = MEDAL_WIDTH * density;
  canvas.height = MEDAL_HEIGHT * density;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(density, 0, 0, density, 0, 0);
  ctx.clearRect(0, 0, MEDAL_WIDTH, MEDAL_HEIGHT);
  const palette = unlocked ? GOLD : SILVER;

  if (unlocked) {
    circle(ctx, 14, 3, 68, 0xffc779, 0.14);
    for (const x of [13, 83]) star(ctx, [x, 33], 4.9, palette.ink);
    for (const x of [19, 77]) {
      star(ctx, [x, 13], 2.5, palette.accent);
      dot(ctx, [x, 58], 1.3, palette.accent);
    }
  }
  dot(ctx, [48, 39], 31.5, 0x16182c);
  gradientDisc(ctx, CENTER, 31, palette.rim);
  dot(ctx, CENTER, 27.8, unlocked ? 0x815347 : 0x464f69);
  gradientDisc(ctx, CENTER, 25.9, palette.face);

  switch (award) {
    case "HomeByMoonlight":
      drawHome(ctx, palette);
      break;
    case "CleanSweep":
      drawBroom(ctx, palette);
      break;
    case "Pacifist":
      drawDove(ctx, palette);
      break;
    case "NightExpress":
      drawStopwatch(ctx, palette);
      break;
    case "AllNineIntact":
      drawCat(ctx, palette);
      break;
    case "EmptyPaws":
      drawEmptyPaws(ctx, palette);
      break;
    case "DoubleFeature":
      drawDoubleFeature(ctx, palette);
      break;
    case "BorrowedLift":
      drawRooftopJump(ctx, palette);
      break;
  }

  if (unlocked) {
    const seal: Point2 = [48, 66.5];
    dot(ctx, seal, 9, 0x352435);
    gradientDisc(ctx, seal, 7.6, [0xffedb5, 0xf2bb72, 0xcd8a4f]);
    line(
      ctx,
      [
        [44.7, 66.3],
        [47, 68.6],
        [51.4, 64.2],
      ],
      0x51303c,
      1.85,
    );
  }
  ctx.globalAlpha = 1;
}

function gradientDisc(
  ctx: CanvasRenderingContext2D,
  [x, y]: Point2,
  r: number,
  colors: readonly [number, number, number],
): void {
  const grad = ctx.createLinearGradient(x - r * 0.45, y - r, x + r * 0.45, y + r);
  grad.addColorStop(0, rgbCss(colors[0]));
  grad.addColorStop(0.58, rgbCss(colors[1]));
  grad.addColorStop(1, rgbCss(colors[2]));
  ctx.globalAlpha = 1;
  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, TAU);
  ctx.fill();
}

function circle(
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

function dot(ctx: CanvasRenderingContext2D, [x, y]: Point2, r: number, ink: number): void {
  circle(ctx, x - r, y - r, 2 * r, ink, 1);
}

function ring(ctx: CanvasRenderingContext2D, [x, y]: Point2, r: number, ink: number): void {
  ctx.globalAlpha = 1;
  ctx.strokeStyle = rgbCss(ink);
  ctx.lineWidth = LINE;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.arc(x, y, r, 0, TAU);
  ctx.stroke();
}

function line(
  ctx: CanvasRenderingContext2D,
  points: readonly Point2[],
  ink: number,
  width: number,
): void {
  if (points.length === 0) return;
  ctx.globalAlpha = 1;
  ctx.strokeStyle = rgbCss(ink);
  ctx.lineWidth = width;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(points[0]![0], points[0]![1]);
  for (let i = 1; i < points.length; i++) {
    ctx.lineTo(points[i]![0], points[i]![1]);
  }
  ctx.stroke();
}

function polygon(ctx: CanvasRenderingContext2D, points: readonly Point2[], ink: number): void {
  if (points.length === 0) return;
  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(ink);
  ctx.beginPath();
  ctx.moveTo(points[0]![0], points[0]![1]);
  for (let i = 1; i < points.length; i++) {
    ctx.lineTo(points[i]![0], points[i]![1]);
  }
  ctx.closePath();
  ctx.fill();
}

function rounded(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
  ink: number,
): void {
  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(ink);
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
  ctx.fill();
}

function star(ctx: CanvasRenderingContext2D, [x, y]: Point2, r: number, ink: number): void {
  const k = r * 0.25;
  polygon(
    ctx,
    [
      [x, y - r],
      [x + k, y - k],
      [x + r, y],
      [x + k, y + k],
      [x, y + r],
      [x - k, y + k],
      [x - r, y],
      [x - k, y - k],
    ],
    ink,
  );
}

function drawHome(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  ctx.globalAlpha = 1;
  ctx.fillStyle = rgbCss(palette.accent);
  ctx.beginPath();
  ctx.moveTo(48.5, 16);
  ctx.bezierCurveTo(40.6, 16.4, 41.3, 27.3, 48.5, 27.3);
  ctx.bezierCurveTo(51.3, 27.3, 53.2, 25.7, 54, 23.5);
  ctx.bezierCurveTo(49.2, 24.2, 46.8, 20.2, 48.5, 16);
  ctx.closePath();
  ctx.fill();

  line(
    ctx,
    [
      [29.5, 42],
      [48, 30],
      [66.5, 42],
    ],
    palette.ink,
    LINE,
  );
  line(
    ctx,
    [
      [34, 39.5],
      [34, 53],
      [62, 53],
      [62, 39.5],
    ],
    palette.ink,
    LINE,
  );
  rounded(ctx, 44, 43, 8, 10, 1.3, palette.accent);
  for (const x of [37.5, 54.5]) rounded(ctx, x, 44, 4, 4, 0.7, palette.ink);
}

function drawBroom(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  for (const x of [32, 64]) {
    ring(ctx, [x, 34], 5.5, palette.accent);
    star(ctx, [x, 34], 2.35, palette.ink);
  }
  line(
    ctx,
    [
      [48, 18],
      [48, 36],
    ],
    palette.accent,
    3.4,
  );
  ctx.strokeStyle = rgbCss(palette.ink);
  ctx.lineWidth = LINE;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(44, 34);
  ctx.lineTo(52, 34);
  ctx.quadraticCurveTo(53.5, 42, 59, 50.5);
  ctx.quadraticCurveTo(48, 55.5, 37, 50.5);
  ctx.quadraticCurveTo(42.5, 42, 44, 34);
  ctx.closePath();
  ctx.stroke();
  line(
    ctx,
    [
      [42.6, 42],
      [53.4, 42],
    ],
    palette.accent,
    1.8,
  );
  for (const [x, end] of [
    [45, 43],
    [48, 48],
    [51, 53],
  ] as const) {
    line(
      ctx,
      [
        [x, 46],
        [end, 51.5],
      ],
      palette.ink,
      1.25,
    );
  }
}

function drawDove(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  ctx.strokeStyle = rgbCss(palette.ink);
  ctx.lineWidth = LINE;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(48, 31.5);
  ctx.bezierCurveTo(42.5, 29, 36.5, 26, 29.5, 22.5);
  ctx.bezierCurveTo(29.5, 35, 35.5, 41, 43.2, 44);
  ctx.quadraticCurveTo(41.7, 49, 37.5, 51);
  ctx.quadraticCurveTo(48, 57, 58.5, 51);
  ctx.quadraticCurveTo(54.3, 49, 52.8, 44);
  ctx.bezierCurveTo(60.5, 41, 66.5, 35, 66.5, 22.5);
  ctx.bezierCurveTo(59.5, 26, 53.5, 29, 48, 31.5);
  ctx.closePath();
  ctx.stroke();
  for (const dir of [-1, 1]) {
    ctx.strokeStyle = rgbCss(palette.accent);
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(48 + dir * 15, 31);
    ctx.quadraticCurveTo(48 + dir * 11, 36.5, 48 + dir * 5, 37.7);
    ctx.stroke();
  }
  dot(ctx, [48, 31.5], 4.4, palette.ink);
  polygon(
    ctx,
    [
      [45.6, 35],
      [50.4, 35],
      [48, 38.7],
    ],
    palette.accent,
  );
}

function drawStopwatch(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  line(
    ctx,
    [
      [44, 19.5],
      [52, 19.5],
    ],
    palette.accent,
    3,
  );
  line(
    ctx,
    [
      [48, 20.5],
      [48, 24],
    ],
    palette.ink,
    LINE,
  );
  line(
    ctx,
    [
      [57, 27],
      [60, 24],
    ],
    palette.accent,
    LINE,
  );
  ring(ctx, [48, 39.5], 15.1, palette.ink);
  for (const pt of [
    [37, 39.5],
    [59, 39.5],
    [48, 50.5],
  ] as const) {
    dot(ctx, [pt[0], pt[1]], 1.15, palette.accent);
  }
  polygon(
    ctx,
    [
      [50, 29],
      [42.1, 40.7],
      [47.2, 40.7],
      [45.8, 48.5],
      [54.1, 36.5],
      [49.1, 36.5],
    ],
    palette.accent,
  );
}

function drawCat(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  for (let i = 0; i < 9; i++) {
    const angle = (i * TAU) / 9 - Math.PI / 2;
    const center: Point2 = [48 + 21.2 * Math.cos(angle), 37 + 21.2 * Math.sin(angle)];
    if (i === 0) star(ctx, center, 2.3, palette.accent);
    else dot(ctx, center, 1.35, palette.accent);
  }
  ctx.strokeStyle = rgbCss(palette.ink);
  ctx.lineWidth = LINE;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(34.5, 40);
  ctx.quadraticCurveTo(34.2, 36, 36.1, 32.8);
  ctx.lineTo(36.1, 25);
  ctx.quadraticCurveTo(36.1, 23.7, 37.5, 24.6);
  ctx.lineTo(43.5, 29.3);
  ctx.quadraticCurveTo(48, 28.2, 52.5, 29.3);
  ctx.lineTo(58.5, 24.6);
  ctx.quadraticCurveTo(59.9, 23.7, 59.9, 25);
  ctx.lineTo(59.9, 32.8);
  ctx.quadraticCurveTo(61.8, 36, 61.5, 40);
  ctx.bezierCurveTo(61.5, 47.4, 55.7, 50.3, 48, 50.3);
  ctx.bezierCurveTo(40.3, 50.3, 34.5, 47.4, 34.5, 40);
  ctx.closePath();
  ctx.stroke();
  for (const x of [42.2, 53.8]) dot(ctx, [x, 39], 1.55, palette.ink);
  polygon(
    ctx,
    [
      [45.9, 43.2],
      [50.1, 43.2],
      [48, 45.8],
    ],
    palette.accent,
  );
}

function drawEmptyPaws(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  ring(ctx, [48, 26], 8.3, palette.accent);
  line(
    ctx,
    [
      [42.2, 31.8],
      [53.8, 20.2],
    ],
    palette.accent,
    LINE,
  );
  for (const [x, y] of [
    [38, 44],
    [44.2, 40.8],
    [51.8, 40.8],
    [58, 44],
  ] as const) {
    dot(ctx, [x, y], 2.5, palette.ink);
  }
  ctx.fillStyle = rgbCss(palette.ink);
  ctx.beginPath();
  ctx.moveTo(48, 46.5);
  ctx.bezierCurveTo(51, 46.5, 51.7, 49, 54.6, 51.7);
  ctx.bezierCurveTo(57.8, 55.2, 53.8, 58.3, 50.6, 56.7);
  ctx.quadraticCurveTo(48, 55.4, 45.4, 56.7);
  ctx.bezierCurveTo(42.2, 58.3, 38.2, 55.2, 41.4, 51.7);
  ctx.bezierCurveTo(44.3, 49, 45, 46.5, 48, 46.5);
  ctx.closePath();
  ctx.fill();
}

function drawDoubleFeature(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  ctx.strokeStyle = rgbCss(palette.ink);
  ctx.lineWidth = LINE;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(48, 22.5);
  ctx.quadraticCurveTo(44, 22.5, 41.3, 19);
  ctx.lineTo(41.3, 23.6);
  ctx.quadraticCurveTo(36, 24.3, 31, 22);
  ctx.quadraticCurveTo(31.7, 29.5, 38.5, 31.5);
  ctx.quadraticCurveTo(45, 31.3, 48, 34.6);
  ctx.quadraticCurveTo(51, 31.3, 57.5, 31.5);
  ctx.quadraticCurveTo(64.3, 29.5, 65, 22);
  ctx.quadraticCurveTo(60, 24.3, 54.7, 23.6);
  ctx.lineTo(54.7, 19);
  ctx.quadraticCurveTo(52, 22.5, 48, 22.5);
  ctx.closePath();
  ctx.stroke();
  for (const x of [44.8, 51.2]) dot(ctx, [x, 27.6], 1.25, palette.ink);
  line(
    ctx,
    [
      [48, 36.8],
      [48, 41.5],
    ],
    palette.accent,
    1.75,
  );
  line(
    ctx,
    [
      [45.4, 39.5],
      [48, 42.1],
      [50.6, 39.5],
    ],
    palette.accent,
    1.75,
  );
  for (const x of [41.7, 54.3]) dot(ctx, [x, 45.8], 2.9, palette.ink);
  ctx.fillStyle = rgbCss(palette.ink);
  ctx.beginPath();
  ctx.moveTo(48, 45);
  ctx.bezierCurveTo(56.6, 45, 57.3, 51.4, 48, 55.8);
  ctx.bezierCurveTo(38.7, 51.4, 39.4, 45, 48, 45);
  ctx.closePath();
  ctx.fill();
  for (const x of [44.8, 51.2]) dot(ctx, [x, 49.2], 1.1, palette.face[1]);
  dot(ctx, [48, 53.7], 1.2, palette.face[1]);
}

function drawRooftopJump(ctx: CanvasRenderingContext2D, palette: MedalPalette): void {
  for (const [left, right, y] of [
    [30, 42, 48.5],
    [54, 66, 43],
  ] as const) {
    ctx.strokeStyle = rgbCss(palette.ink);
    ctx.lineWidth = LINE;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(left, 55);
    ctx.lineTo(left, y);
    ctx.lineTo(right, y);
    ctx.lineTo(right, 55);
    ctx.closePath();
    ctx.stroke();
    line(
      ctx,
      [
        [left - 1.8, y],
        [right + 1.8, y],
      ],
      palette.ink,
      LINE,
    );
    for (const x of [left + 3.6, right - 3.6]) dot(ctx, [x, y + 3.25], 0.9, palette.accent);
  }
  dot(ctx, [36, 45], 1.55, palette.accent);
  ctx.strokeStyle = rgbCss(palette.accent);
  ctx.lineWidth = LINE;
  ctx.beginPath();
  ctx.moveTo(36, 41);
  ctx.bezierCurveTo(41, 24, 54, 23.5, 62.5, 35.7);
  ctx.stroke();
  line(
    ctx,
    [
      [62.5, 30.5],
      [62.5, 35.7],
      [57.4, 35],
    ],
    palette.accent,
    LINE,
  );
}
