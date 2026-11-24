import {
  clamp,
  POSTER_FONT,
  PURPLE,
  remEuclid,
  rgbCss,
  timeLabel,
  type ArtPath,
  type CheckpointPalette,
  type Point2,
  type PosterShape,
  type PosterText,
  type Score,
  type StatIcon,
} from "./types";

const TAU = Math.PI * 2;
const FRAC_PI_2 = Math.PI / 2;
const FRAC_PI_4 = Math.PI / 4;

function polygon(points: Point2[], ink: number, opacity: number): ArtPath {
  return { points, ink, opacity };
}

function ellipse(
  x: number,
  y: number,
  rx: number,
  ry: number,
  ink: number,
  opacity: number,
): ArtPath {
  const points: Point2[] = [];
  for (let i = 0; i < 128; i++) {
    const angle = (i * TAU) / 128;
    points.push([x + rx * Math.cos(angle), y + ry * Math.sin(angle)]);
  }
  return polygon(points, ink, opacity);
}

export function shelterShapes(
  x: number,
  y: number,
  home: boolean,
  lit: boolean,
  size: number,
): PosterShape[] {
  return shelterWithPalette(x, y, home, lit, size, {
    wall: 0x4b3b4c,
    coping: 0x877181,
    frame: 0x937683,
  });
}

export function checkpointShapes(
  x: number,
  y: number,
  lit: boolean,
  size: number,
  palette: CheckpointPalette,
): PosterShape[] {
  return shelterWithPalette(x, y, false, lit, size, palette);
}

function shelterWithPalette(
  x: number,
  y: number,
  home: boolean,
  lit: boolean,
  size: number,
  palette: CheckpointPalette,
): PosterShape[] {
  const shapes: PosterShape[] = [];
  const add = (dx: number, dy: number, w: number, h: number, ink: number, radius: number) => {
    shapes.push({
      rect: [x + dx * size, y + dy * size, w * size, h * size],
      ink,
      opacity: 1,
      radius: radius * size,
    });
  };
  const light = lit ? 0xffdba0 : 0x242535;
  if (home) {
    add(-138, -110, 196, 110, 0x514052, 4);
    add(-134, -106, 188, 5, 0x9b7d87, 1);
    add(-132, -99, 4, 99, 0x705466, 0);
    add(50, -99, 4, 99, 0x705466, 0);
    add(-138, -8, 196, 8, 0x695163, 0);
    add(-116, -150, 17, 41, 0x6c5361, 1);
    add(-120, -154, 25, 5, 0xb69a9c, 2);

    add(-13, -82, 44, 82, 0xb39492, 22);
    add(-13, -41, 44, 41, 0xb39492, 0);
    add(-8, -77, 34, 77, 0x3b3448, 17);
    add(-8, -39, 34, 39, 0x3b3448, 0);
    add(-1, -67, 20, 31, light, 10);
    add(-1, -50, 20, 14, light, 0);
    add(-4, -28, 26, 23, 0x59475c, 2);
    add(18, -34, 3, 3, 0xc7a69b, 1.5);
    add(-18, -2, 54, 2, 0xc7a69b, 0);

    add(-122, -91, 100, 74, 0xb39492, 14);
    add(-117, -86, 90, 64, lit ? 0xf2c994 : 0x414056, 10);
    add(-115, -37, 86, 15, lit ? 0xc39c88 : 0x5d506b, 4);
    add(-104, -33, 64, 10, lit ? 0xb08caa : 0x99839e, 5);
    add(-125, -22, 108, 6, 0xc7a69b, 2);
    add(-117, -16, 92, 4, 0x7c6076, 1);
  } else {
    add(-47, -86, 112, 86, palette.wall, 6);
    add(-52, -92, 122, 8, palette.coping, 3);
    add(-17, -74, 52, 74, palette.frame, 25);
    add(-17, -37, 52, 37, palette.frame, 0);
    add(-13, -70, 44, 70, light, 22);
    add(-13, -32, 44, 32, light, 0);
    add(41, -42, 15, 20, palette.frame, 5);
    add(44, -39, 9, 14, light, 3);
    add(-20, -2, 58, 2, 0xb89a92, 1);
  }
  for (const dx of home ? [-137, 39] : [-38]) {
    add(dx, -17, 16, 17, 0x84616a, 4);
    add(dx - 4, -19, 24, 5, 0xb08787, 2);
    add(dx - 5, -33, 26, 23, 0x687d70, 11);
    add(dx + 2, -38, 14, 24, 0x81947a, 7);
    if (home) {
      add(dx + 1, -37, 5, 5, 0xc4a3be, 2.5);
      add(dx + 11, -29, 5, 5, 0xc4a3be, 2.5);
    }
  }
  if (home && !lit) {
    const [catX, catY] = homeCatPosition(x, y, size);
    shapes.push(...restingCatShapes(catX, catY, 0x05070d, size));
  }
  return shapes;
}

function homeCatPosition(x: number, y: number, size: number): Point2 {
  return [x - 81 * size, y - 32 * size];
}

export function homeSleepPaths(x: number, y: number, time: number, size: number): ArtPath[] {
  const [catX, catY] = homeCatPosition(x, y, size);
  return sleepPathsWithRise(catX, catY, time, size, 24);
}

export function homeRoofPaths(x: number, y: number, size: number, lit: boolean): ArtPath[] {
  const roofs: [Point2[], number][] = [
    [
      [
        [-150, -108],
        [-40, -155],
        [70, -108],
      ],
      0xaf9297,
    ],
    [
      [
        [-137, -112],
        [-40, -150],
        [57, -112],
      ],
      0x394353,
    ],
    [
      [
        [-124, -113],
        [-40, -145],
        [44, -113],
      ],
      0x465567,
    ],
  ];
  const paths: ArtPath[] = roofs.map(([points, ink]) =>
    polygon(
      points.map(([dx, dy]) => [x + dx * size, y + dy * size]),
      ink,
      1,
    ),
  );
  paths.push(ellipse(x - 40 * size, y - 121 * size, 10 * size, 10 * size, 0xb39492, 1));
  paths.push(
    ellipse(x - 40 * size, y - 121 * size, 7 * size, 7 * size, lit ? 0xffdba0 : 0x242535, 1),
  );
  return paths;
}

export function restingCatShapes(x: number, y: number, ink: number, size: number): PosterShape[] {
  const defs: [number, number, number, number, number][] = [
    [-11, -14, 37, 14, 7],
    [13, -19, 16, 16, 8],
    [20, -20, 4, 7, 1],
  ];
  return defs.map(([dx, dy, w, h, radius]) => ({
    rect: [x + dx * size, y + dy * size, w * size, h * size],
    ink,
    opacity: 1,
    radius: radius * size,
  }));
}

export function sleepPaths(x: number, y: number, time: number, size: number): ArtPath[] {
  return sleepPathsWithRise(x, y, time, size, 35);
}

const Z_GLYPH: readonly Point2[] = [
  [0, 0],
  [1, 0],
  [1, 0.18],
  [0.28, 0.82],
  [1, 0.82],
  [1, 1],
  [0, 1],
  [0, 0.82],
  [0.72, 0.18],
  [0, 0.18],
];

function sleepPathsWithRise(
  x: number,
  y: number,
  time: number,
  size: number,
  rise: number,
): ArtPath[] {
  const result: ArtPath[] = [];
  for (let i = 0; i < 3; i++) {
    const t = remEuclid(time * 0.43 + i / 3, 1);
    const d = (5 + t * 3) * size;
    const left = x + (23 + t * 13) * size;
    const top = y - (29 + t * rise) * size;
    const opacity = ((Math.min(t * 8, 1) * Math.min(1 - t, 0.3)) / 0.3) * 0.8;
    result.push(
      polygon(
        Z_GLYPH.map(([dx, dy]) => [left + dx * d, top + dy * d]),
        0xd4c4df,
        opacity,
      ),
    );
  }
  return result;
}

const MOON_TERMINATORS = [0.64, 0, -0.72] as const;

export function moonPaths(x: number, y: number, diameter: number, level: number): ArtPath[] {
  const terminator = MOON_TERMINATORS[Math.min(level, 2)] ?? -0.72;
  return moonWithTerminator(x, y, diameter, terminator);
}

export function fullMoonPaths(x: number, y: number, diameter: number): ArtPath[] {
  return moonWithTerminator(x, y, diameter, -1);
}

function moonWithTerminator(x: number, y: number, diameter: number, terminator: number): ArtPath[] {
  const r = diameter * 0.5;
  const cx = x + r;
  const cy = y + r;
  const illumination = (1 - terminator) * 0.5;
  const paths: ArtPath[] = [
    ellipse(cx, cy, r * 1.8, r * 1.8, 0xffdba0, 0.025 * illumination),
    ellipse(cx, cy, r * 1.35, r * 1.35, 0xffdba0, 0.04 * illumination),
    ellipse(cx, cy, r, r, 0x292c40, 1),
  ];
  const litPoints: Point2[] = [];
  for (let i = 0; i <= 128; i++) {
    const angle = -FRAC_PI_2 + (i * Math.PI) / 128;
    litPoints.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)]);
  }
  for (let i = 0; i <= 128; i++) {
    const angle = FRAC_PI_2 - (i * Math.PI) / 128;
    litPoints.push([cx + terminator * r * Math.cos(angle), cy + r * Math.sin(angle)]);
  }
  paths.push(polygon(litPoints, 0xf3ddba, 1));
  const craters: readonly [number, number, number][] = [
    [0.79, -0.2, 0.1],
    [0.64, 0.45, 0.13],
    [0.28, -0.57, 0.11],
    [-0.35, 0.22, 0.17],
  ];
  for (const [dx, dy, radius] of craters) {
    const crater = ellipse(cx + dx * r, cy + dy * r, radius * r, radius * r, 0xbda789, 0.24);
    for (const point of crater.points) {
      const yy = clamp((point[1] - cy) / r, -1, 1);
      const edge = Math.sqrt(1 - yy * yy) * r;
      point[0] = clamp(point[0], cx + terminator * edge, cx + edge);
    }
    paths.push(crater);
  }
  return paths;
}

function outline(points: readonly Point2[], ink: number): ArtPath {
  return polygon(
    points.map(([x, y]) => [x, y]),
    ink,
    1,
  );
}

function appendCurve(points: Point2[], c1: Point2, c2: Point2, end: Point2): void {
  const start = points[points.length - 1]!;
  for (let i = 1; i <= 24; i++) {
    const t = i / 24;
    const u = 1 - t;
    points.push([
      u * u * u * start[0] + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t * t * t * end[0],
      u * u * u * start[1] + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t * t * t * end[1],
    ]);
  }
}

function capsule(a: Point2, b: Point2, width: number, ink: number): ArtPath {
  const angle = Math.atan2(b[1] - a[1], b[0] - a[0]);
  const r = width * 0.5;
  const points: Point2[] = [];
  for (const [center, offset] of [
    [b, -FRAC_PI_2],
    [a, FRAC_PI_2],
  ] as const) {
    for (let i = 0; i <= 32; i++) {
      const theta = angle + offset + (i * Math.PI) / 32;
      points.push([center[0] + r * Math.cos(theta), center[1] + r * Math.sin(theta)]);
    }
  }
  return polygon(points, ink, 1);
}

function roundedTriangle(ink: number): ArtPath {
  const points: Point2[] = [[10, 11.7]];
  const segments: readonly [Point2, Point2, Point2][] = [
    [
      [10.3, 11.7],
      [11.4, 13.6],
      [11.4, 13.9],
    ],
    [
      [11.4, 14.3],
      [8.6, 14.3],
      [8.6, 13.9],
    ],
    [
      [8.6, 13.6],
      [9.7, 11.7],
      [10, 11.7],
    ],
  ];
  for (const [c1, c2, end] of segments) {
    appendCurve(points, c1, c2, end);
  }
  return polygon(points, ink, 1);
}

export function statPaths(
  kind: StatIcon,
  x: number,
  y: number,
  size: number,
  background: number,
): ArtPath[] {
  let paths: ArtPath[];
  switch (kind) {
    case "coin":
      paths = [
        ellipse(10, 10, 9, 9, PURPLE, 1),
        ellipse(10, 10, 6.5, 6.5, background, 1),
        capsule([10, 6.5], [10, 13.5], 2.2, PURPLE),
      ];
      break;
    case "deaths": {
      const right: Point2[] = [[10, 1.3]];
      const curves: readonly [Point2, Point2, Point2][] = [
        [
          [15.3, 1.3],
          [18.2, 4.3],
          [18.2, 8.4],
        ],
        [
          [18.2, 11.5],
          [17.2, 13.1],
          [15.5, 14],
        ],
        [
          [15, 14.3],
          [14.7, 14.7],
          [14.7, 15.4],
        ],
        [
          [14.7, 18.3],
          [13.8, 18.8],
          [10, 18.8],
        ],
      ];
      for (const [c1, c2, end] of curves) {
        appendCurve(right, c1, c2, end);
      }
      const left: Point2[] = [];
      for (let i = right.length - 2; i >= 0; i--) {
        const p = right[i]!;
        left.push([20 - p[0], p[1]]);
      }
      right.push(...left);
      paths = [
        polygon(right, PURPLE, 1),
        ellipse(6.7, 9.1, 2.05, 2.25, background, 1),
        ellipse(13.3, 9.1, 2.05, 2.25, background, 1),
        roundedTriangle(background),
        capsule([8.6, 16.4], [8.6, 17.4], 0.9, background),
        capsule([11.4, 16.4], [11.4, 17.4], 0.9, background),
      ];
      break;
    }
    case "kills": {
      paths = [];
      for (const angle of [-FRAC_PI_4, FRAC_PI_4]) {
        const cos = Math.cos(angle);
        const sin = Math.sin(angle);
        const sword = [
          outline(
            [
              [10, 0.5],
              [11.6, 3.4],
              [11.25, 12.4],
              [8.75, 12.4],
              [8.4, 3.4],
            ],
            PURPLE,
          ),
          capsule([6.6, 12.9], [13.4, 12.9], 1.9, PURPLE),
          capsule([10, 13.4], [10, 17.6], 2.1, PURPLE),
          ellipse(10, 18, 1.5, 1.5, PURPLE, 1),
        ];
        for (const p of sword) {
          for (const pt of p.points) {
            const dx = pt[0] - 10;
            const dy = pt[1] - 10;
            pt[0] = 10 + dx * cos - dy * sin;
            pt[1] = 10 + dx * sin + dy * cos;
          }
        }
        paths.push(...sword);
      }
      break;
    }
    case "time":
      paths = [
        ellipse(10, 10, 9, 9, PURPLE, 1),
        ellipse(10, 10, 6.5, 6.5, background, 1),
        outline(
          [
            [9, 5],
            [11, 5],
            [11, 9.5],
            [14, 11.5],
            [13, 13],
            [9, 10.5],
          ],
          PURPLE,
        ),
      ];
      break;
  }
  const opticalScale =
    kind === "coin" ? 0.95 : kind === "deaths" ? 0.93 : kind === "kills" ? 1.04 : 0.95;
  for (const path of paths) {
    for (const point of path.points) {
      point[0] = x + ((10 + (point[0] - 10) * opticalScale) * size) / 20;
      point[1] = y + ((10 + (point[1] - 10) * opticalScale) * size) / 20;
    }
  }
  return paths;
}

/** Authored 900x540 poster shapes shared by the results view and 3600x2160 PNG export. */
export function posterShapes(): PosterShape[] {
  const shapes: PosterShape[] = [];
  const add = (
    rect: [number, number, number, number],
    ink: number,
    opacity: number,
    radius: number,
  ) => {
    shapes.push({ rect, ink, opacity, radius });
  };
  add([0, 0, 900, 540], 0x171a2d, 1, 0);
  for (let i = 0; i < 40; i++) {
    const x = ((i * 137 + 33) % 820) + 40;
    const y = ((i * 47 + 13) % 240) + 38;
    if (x < 550 && y < 175) continue;
    const d = i % 7 === 0 ? 2.5 : 1.3;
    add([x, y, d, d], 0xf3ddba, 0.25, d * 0.5);
  }
  for (let layer = 0; layer < 2; layer++) {
    for (let i = 0; i < 12; i++) {
      const x = i * 82 - 18 + layer * 29;
      const roof = 245 + ((i * 31 + layer * 19) % 65);
      const ink = layer === 0 ? 0x24273c : 0x303044;
      add([x, roof, 73, 365 - roof], ink, 1, 2);
      add([x + 10, roof - 7, 50, 8], ink, 1, 0);
      if (i % 3 === 0) {
        add([x + 24, roof - 25, 15, 20], ink, 1, 2);
      }
      for (let row = 0; row < 3; row++) {
        for (let col = 0; col < 3; col++) {
          const y = roof + 13 + row * 17;
          if (y > 352) continue;
          add([x + 12 + col * 18, y, 5, 8], 0xe2be91, (i + row + col) % 3 === 0 ? 0.7 : 0.12, 0.5);
        }
      }
    }
  }
  add([576, 343, 248, 12], 0x393447, 1, 0);
  add([570, 337, 260, 7], 0x967784, 1, 2);
  for (const shape of shelterShapes(740, 337, true, false, 1.1)) {
    shapes.push(shape);
  }
  for (const rect of [
    [20, 20, 860, 0.8],
    [20, 519, 860, 0.8],
    [20, 20, 0.8, 500],
    [879, 20, 0.8, 500],
  ] as const) {
    add([rect[0], rect[1], rect[2], rect[3]], 0xc5a07b, 0.35, 0);
  }
  add([36, 354, 828, 142], 0x222338, 1, 8);
  add([52, 354, 796, 1], 0xd8b48c, 0.55, 0);
  return shapes;
}

export function posterStaticPaths(): ArtPath[] {
  const paths = homeRoofPaths(740, 337, 1.1, false);
  const cols: readonly [StatIcon, number][] = [
    ["coin", 139.5],
    ["deaths", 346.5],
    ["kills", 553.5],
    ["time", 760.5],
  ];
  for (const [kind, center] of cols) {
    paths.push(...statPaths(kind, center - 12, 371, 24, 0x222338));
  }
  return paths;
}

export function posterPaths(time: number): ArtPath[] {
  return [...posterStaticPaths(), ...homeSleepPaths(740, 337, time, 1.1)];
}

export function posterText(score: Score): PosterText[] {
  const text: PosterText[] = [
    {
      label: "ROOFTOPS",
      x: 44,
      y: 42,
      size: 14,
      ink: 0xd4b89c,
      bold: true,
      centeredWidth: null,
    },
    {
      label: "Home at last",
      x: 41,
      y: 76,
      size: 52,
      ink: 0xf3ddba,
      bold: true,
      centeredWidth: null,
    },
  ];
  if (score.personalBest !== null && score.seconds > score.personalBest) {
    text.push({
      label: `personal best: ${timeLabel(score.personalBest)}`,
      x: 44,
      y: 502,
      size: 10,
      ink: 0x96899f,
      bold: false,
      centeredWidth: null,
    });
  }
  const columns: readonly [string, string][] = [
    ["COINS", String(score.coins)],
    ["DEATHS", String(score.deaths)],
    ["ENEMIES DEFEATED", String(score.kills)],
    ["COMPLETION TIME", timeLabel(score.seconds)],
  ];
  columns.forEach(([label, value], column) => {
    const x = 36 + column * 207;
    text.push({
      label,
      x,
      y: 460,
      size: 11,
      ink: 0xb9a5b4,
      bold: true,
      centeredWidth: 207,
    });
    text.push({
      label: value,
      x,
      y: 401,
      size: Math.min(44, 172 / (Math.max(1, value.length) * 0.7)),
      ink: 0xf3ddba,
      bold: true,
      centeredWidth: 207,
    });
  });
  return text;
}

export function drawRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
): void {
  if (w <= 0 || h <= 0) return;
  const r = Math.min(radius, w * 0.5, h * 0.5);
  if (r <= 0) {
    ctx.fillRect(x, y, w, h);
    return;
  }
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
  ctx.fill();
}

export function drawPosterShape(
  ctx: CanvasRenderingContext2D,
  shape: PosterShape,
  scale: number,
): void {
  const [x, y, w, h] = shape.rect;
  if (w <= 0 || h <= 0 || shape.opacity <= 0) return;
  ctx.globalAlpha = shape.opacity;
  ctx.fillStyle = rgbCss(shape.ink);
  drawRoundedRect(ctx, x * scale, y * scale, w * scale, h * scale, shape.radius * scale);
}

export function drawArtPaths(
  ctx: CanvasRenderingContext2D,
  paths: readonly ArtPath[],
  scale: number,
): void {
  let lastInk = -1;
  let lastAlpha = -1;
  for (const shape of paths) {
    const pts = shape.points;
    if (pts.length < 3 || shape.opacity <= 0) continue;
    if (shape.ink !== lastInk) {
      ctx.fillStyle = rgbCss(shape.ink);
      lastInk = shape.ink;
    }
    if (shape.opacity !== lastAlpha) {
      ctx.globalAlpha = shape.opacity;
      lastAlpha = shape.opacity;
    }
    ctx.beginPath();
    const first = pts[0]!;
    ctx.moveTo(first[0] * scale, first[1] * scale);
    for (let i = 1; i < pts.length; i++) {
      const p = pts[i]!;
      ctx.lineTo(p[0] * scale, p[1] * scale);
    }
    ctx.closePath();
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}

/** Paint the 900x540 poster onto a Canvas 2D at an arbitrary scale (e.g. 4 for 3600x2160 export). */
export function renderPosterToCanvas(
  ctx: CanvasRenderingContext2D,
  score: Score,
  scale: number,
  sleepTime: number,
): void {
  const shapes = posterShapes();
  const bg = shapes[0];
  if (bg) {
    drawPosterShape(ctx, bg, scale);
  }
  drawArtPaths(ctx, fullMoonPaths(654, 130, 122), scale);
  for (let i = 1; i < shapes.length; i++) {
    drawPosterShape(ctx, shapes[i]!, scale);
  }
  ctx.globalAlpha = 1;
  drawArtPaths(ctx, posterPaths(sleepTime), scale);
  ctx.globalAlpha = 1;
  ctx.textBaseline = "top";
  for (const item of posterText(score)) {
    ctx.fillStyle = rgbCss(item.ink);
    ctx.font = `${item.bold ? "700" : "400"} ${item.size * scale}px ${POSTER_FONT}`;
    if (item.centeredWidth !== null) {
      ctx.textAlign = "center";
      ctx.fillText(item.label, (item.x + item.centeredWidth * 0.5) * scale, item.y * scale);
    } else {
      ctx.textAlign = "left";
      ctx.fillText(item.label, item.x * scale, item.y * scale);
    }
  }
  ctx.textAlign = "left";
}

export async function downloadScorecardPng(score: Score): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 3600;
  canvas.height = 2160;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Could not create scorecard canvas");
  renderPosterToCanvas(ctx, score, 4, 1.2);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((result) => {
      if (result) resolve(result);
      else reject(new Error("Could not encode scorecard PNG"));
    }, "image/png");
  });
  const url = URL.createObjectURL(blob);
  const filename = "Rooftops-score.png";
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
  return filename;
}
