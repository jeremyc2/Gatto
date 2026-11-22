import { ease, progress, randomFrom, type PortalFrame } from "./geometry";

interface CutPoint {
  x: number;
  y: number;
  length: number;
  weight: number;
  fray: number;
}
interface Chip {
  x: number;
  start: number;
  life: number;
  size: number;
  vx: number;
  vy: number;
  angle: number;
  spin: number;
  color: string;
}

const chipColors = ["#dbb890", "#bea080", "#816650", "#af8461", "#ebcc9a"];
const random = randomFrom(1839);
const chips: Chip[] = Array.from({ length: 190 }, () => {
  const x = (random() - 0.5) * 2;
  return {
    x,
    start: 0.18 + Math.abs(x) * 0.39 + random() * 0.36,
    life: 0.28 + random() * 0.81,
    size: 0.45 + Math.pow(random(), 3) * 3.6,
    vx: (random() - 0.5) * 125 + x * 30,
    vy: -14 - random() * 106,
    angle: random() * Math.PI,
    spin: (random() - 0.5) * 19,
    color: chipColors[Math.floor(random() * chipColors.length)]!,
  };
});

// The hand-drawn contour from the original paper seam. Both faces start on
// this one edge, with the 24 px midpoint placed at the center of the stage.
const contour = [
  [0, 24],
  [12, 25],
  [18, 22],
  [28, 27],
  [35, 24],
  [48, 29],
  [59, 28],
  [63, 34],
  [71, 31],
  [79, 35],
  [87, 29],
  [93, 30],
  [101, 20],
  [105, 22],
  [119, 17],
  [127, 21],
  [138, 19],
  [148, 23],
  [155, 21],
  [163, 28],
  [168, 24],
  [172, 25],
  [181, 17],
  [188, 20],
  [195, 12],
  [201, 16],
  [212, 13],
  [219, 18],
  [227, 17],
  [234, 24],
  [246, 26],
  [252, 22],
  [263, 31],
  [270, 29],
  [276, 40],
  [280, 35],
  [286, 38],
  [295, 30],
  [306, 32],
  [312, 29],
  [325, 34],
  [331, 31],
  [342, 36],
  [346, 31],
  [354, 33],
  [361, 26],
  [369, 28],
  [376, 23],
  [383, 26],
  [390, 20],
  [405, 22],
  [414, 18],
  [425, 21],
  [431, 17],
  [441, 19],
  [448, 14],
  [456, 17],
  [463, 13],
  [471, 19],
  [477, 17],
  [487, 24],
  [496, 21],
  [505, 27],
  [517, 25],
  [524, 29],
  [530, 25],
  [539, 31],
  [547, 27],
  [553, 37],
  [557, 33],
  [561, 42],
  [566, 37],
  [574, 39],
  [583, 31],
  [587, 32],
  [593, 26],
  [605, 28],
  [613, 23],
  [618, 26],
  [627, 21],
  [633, 25],
  [640, 23],
  [649, 30],
  [657, 26],
  [666, 29],
  [675, 22],
  [684, 25],
  [690, 21],
  [702, 23],
  [707, 18],
  [714, 21],
  [721, 17],
  [729, 21],
  [738, 19],
  [746, 24],
  [752, 21],
  [759, 25],
  [768, 24],
] as const;

export function fissureProfile(width: number, height: number, radius: number): CutPoint[] {
  const noise = randomFrom(773);
  const scale = width <= 760 ? 5 / 6 : 1;
  const reach = Math.max(radius * 2.65, width * 0.43);
  const points: CutPoint[] = [];
  for (let origin = 0; origin < width; origin += 768 * scale) {
    for (const [px, py] of contour.slice(origin === 0 ? 0 : 1)) {
      const x = origin + px * scale;
      const offset = x - width / 2;
      points.push({
        x,
        y: (py - 24) * scale * Math.min(1, height / 365),
        length: Math.abs(offset) / (width / 2),
        weight: Math.exp(-Math.pow(Math.abs(offset / reach), 2.24)),
        fray: noise(),
      });
      if (x >= width) return points;
    }
  }
  return points;
}

function edgePaths(frame: PortalFrame, points: CutPoint[]) {
  const upper = new Path2D();
  const lower = new Path2D();
  const top = new Path2D();
  const bottom = new Path2D();
  const fibers = new Path2D();
  const { width, height, time, y, radius } = frame;
  const lift = Math.min(height * 0.43, radius * 1.45 + 6);
  upper.moveTo(0, 0);
  lower.moveTo(0, height);
  for (const point of points) {
    // Stress travels from the center and the lips separate along the same cut.
    const localTime = time - point.length * 0.37;
    const split = ease(0.18, 0.82, localTime);
    const open = ease(0.39, 1.56, localTime);
    const drop = ease(0.42, 1.66, localTime);
    const rebound =
      Math.sin(Math.max(0, localTime - 0.92) * 13) *
      (1 - ease(0.92, 1.75, localTime)) *
      ease(0.92, 1.06, localTime);
    const offset = point.x - frame.x;
    const clearance = radius * 1.17 + 3;
    const minimum =
      Math.abs(offset) < clearance
        ? Math.sqrt(clearance ** 2 - offset ** 2) + Math.abs(point.y) + radius * 0.092
        : 0;
    const spread = Math.max(lift * (0.18 + 0.82 * point.weight), minimum);
    const seam = y + point.y + split * radius * 0.046 * Math.sin((point.x / width) * 6.3 + 0.43);
    const curl = radius * 0.043 * Math.sin(point.x / (radius * 1.3));
    const grain = (point.fray - 0.5) * split * 1.7;
    const upperY = seam - (open * 0.955 + 0.013 * rebound) * spread - 1.3 * split + curl * open;
    const lowerY =
      seam + (drop + 0.018 * rebound) * spread + 1.3 * split - curl * drop * 0.5 + grain;
    upper.lineTo(point.x, upperY);
    lower.lineTo(point.x, lowerY);
    if (point.x === 0) {
      top.moveTo(point.x, upperY);
      bottom.moveTo(point.x, lowerY);
    } else {
      top.lineTo(point.x, upperY);
      bottom.lineTo(point.x, lowerY);
    }
    if (split > 0.18 && point.fray > 0.49) {
      const length = split * (point.fray - 0.35) * 8;
      const drift = 2.3 * (point.fray - 0.74);
      const edge = point.fray > 0.72 ? upperY : lowerY;
      const bend = point.fray > 0.72 ? length : -length;
      fibers.moveTo(point.x - 1, edge);
      fibers.quadraticCurveTo(point.x + drift, edge + bend * 0.3, point.x + drift * 2, edge + bend);
    }
  }
  const end = Math.max(width, points.at(-1)?.x ?? width);
  upper.lineTo(end, 0);
  lower.lineTo(end, height);
  upper.closePath();
  lower.closePath();
  return { upper, lower, top, bottom, fibers };
}

function tintFace(
  ctx: CanvasRenderingContext2D,
  path: Path2D,
  fill: string,
  glow: CanvasGradient,
  width: number,
  height: number,
  shade: number,
) {
  ctx.fillStyle = fill;
  ctx.fill(path);
  ctx.save();
  ctx.clip(path);
  ctx.globalAlpha = shade;
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, width, height);
  ctx.restore();
}

export function drawStone(
  ctx: CanvasRenderingContext2D,
  frame: PortalFrame,
  stone: CanvasPattern,
  dark: boolean,
) {
  const { width, height, x, y, radius } = frame;
  ctx.fillStyle = stone;
  ctx.fillRect(0, 0, width, height);

  // Low, warm sunlight across the watercolor plaster, pulled toward the
  // courtyard's ochre wall and olive shadows.
  const wash = ctx.createLinearGradient(width * 0.24, 0, width * 0.68, height);
  wash.addColorStop(0, dark ? "#45423280" : "#f9df9a55");
  wash.addColorStop(0.42, dark ? "#3e312da0" : "#e8bc7c20");
  wash.addColorStop(1, dark ? "#272c23c0" : "#7e715d48");
  ctx.fillStyle = wash;
  ctx.fillRect(0, 0, width, height);

  const hollow = ctx.createRadialGradient(x, y, radius * 0.2, x, y, radius * 3.0);
  hollow.addColorStop(0, dark ? "#201627d9" : "#65433389");
  hollow.addColorStop(0.5, dark ? "#24211b82" : "#765c3d35");
  hollow.addColorStop(1, "#31222d00");
  ctx.fillStyle = hollow;
  ctx.fillRect(0, 0, width, height);
}

export function drawFissure(
  ctx: CanvasRenderingContext2D,
  frame: PortalFrame,
  points: CutPoint[],
  colors: { upper: string; lower: string; dark: boolean },
  light: number,
  drawDepth?: () => void,
) {
  const { width, height, x, y, radius, time } = frame;
  const { upper, lower, top, bottom, fibers } = edgePaths(frame, points);

  if (time <= 0.18) {
    // One face over an opaque ground gives the two paper colors a single
    // shared boundary, even on a fractional backing-store scale.
    ctx.fillStyle = colors.lower;
    ctx.fillRect(0, 0, width, height);
    ctx.fillStyle = colors.upper;
    ctx.fill(upper);
    ctx.lineWidth = 1.25;
    ctx.strokeStyle = colors.dark ? "#795e497a" : "#a98a5a8c";
    ctx.stroke(top);
    return;
  }

  // A broad local spill lights the stone under the ring. The ring is made
  // from live filaments and sparks.
  const spill = ctx.createRadialGradient(x - radius * 0.05, y, radius * 0.25, x, y, radius * 2.45);
  spill.addColorStop(0, `rgba(255, 174, 71, ${0.3 * light})`);
  spill.addColorStop(0.52, `rgba(238, 118, 40, ${0.22 * light})`);
  spill.addColorStop(1, "#e76c2300");
  ctx.fillStyle = spill;
  ctx.fillRect(0, 0, width, height);
  drawDepth?.();

  // The paper-colored page and footer are the front faces of the cut. Near
  // their rims they pick up shadow and light; their outer edges match the DOM.
  const upperShade = ctx.createLinearGradient(0, 0, 0, height);
  upperShade.addColorStop(0, "#30231c00");
  upperShade.addColorStop(0.27, "#30231c00");
  upperShade.addColorStop(0.51, colors.dark ? "#1a1314a0" : "#92714635");
  const lowerShade = ctx.createLinearGradient(0, 0, 0, height);
  lowerShade.addColorStop(0.49, colors.dark ? "#1a1314a0" : "#92714647");
  lowerShade.addColorStop(0.74, "#58412a00");
  lowerShade.addColorStop(1, "#58412a00");

  const separation = ease(0.18, 0.58, time);
  ctx.save();
  ctx.lineJoin = "round";
  ctx.globalAlpha = separation;
  for (const [width, dark, light] of [
    [21, "#1a141521", "#68432b13"],
    [10, "#1a14153d", "#68432b2a"],
    [3.1, "#211a18a8", "#7d573d6c"],
  ] as const) {
    ctx.lineWidth = width;
    ctx.strokeStyle = colors.dark ? dark : light;
    ctx.stroke(top);
    ctx.stroke(bottom);
  }
  ctx.lineWidth = 0.72;
  ctx.strokeStyle = colors.dark ? "#d5ac7970" : "#fff0c2a6";
  ctx.stroke(fibers);
  ctx.globalAlpha = 1;
  tintFace(ctx, upper, colors.upper, upperShade, width, height, separation);
  tintFace(ctx, lower, colors.lower, lowerShade, width, height, separation);
  ctx.globalAlpha = 1 - separation;
  ctx.lineWidth = 1.25;
  ctx.strokeStyle = colors.dark ? "#795e497a" : "#a98a5a8c";
  ctx.stroke(top);
  ctx.globalAlpha = 1;

  const rim = ctx.createRadialGradient(x, y, radius * 0.55, x, y, radius * 2.3);
  const brightness = light * 0.63;
  rim.addColorStop(0, `rgba(255, 207, 134, ${brightness})`);
  rim.addColorStop(0.4, `rgba(255, 184, 95, ${brightness * 0.65})`);
  rim.addColorStop(1, "#ee823800");
  ctx.lineWidth = 0.85;
  ctx.strokeStyle = rim;
  ctx.stroke(top);
  ctx.stroke(bottom);
  ctx.restore();

  if (time < 0.13 || time > 1.62) return;
  const shardReach = Math.min(width * 0.48, radius * 3.3);
  for (const chip of chips) {
    const age = time - chip.start;
    if (age <= 0 || age >= chip.life) continue;
    const px = x + chip.x * shardReach + chip.vx * age;
    const py = y - 13 * ease(0.27, 0.88, time) + chip.vy * age + 155 * age * age;
    const rotation = chip.angle + age * chip.spin;
    const size = chip.size * Math.min(1, radius / 98);
    const dx = Math.cos(rotation) * size;
    const dy = Math.sin(rotation) * size;
    ctx.globalAlpha = (1 - progress(chip.life * 0.45, chip.life, age)) * 0.88;
    ctx.fillStyle = chip.color;
    ctx.beginPath();
    ctx.moveTo(px + dx, py + dy);
    ctx.lineTo(px - dx - dy * 0.5, py - dy + dx * 0.5);
    ctx.lineTo(px + dy * 0.5, py - dx * 0.62);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}
