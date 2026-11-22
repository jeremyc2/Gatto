import { TAU, clamp, ease, mix, randomFrom, type PortalFrame } from "./geometry";

const ignition = 0.66;
const palette = ["#c35428", "#f26927", "#ff9a43", "#ffc46b", "#ffedb1", "#fffbed"];
const strokes = [0.63, 0.7, 0.87, 1.04, 1.16, 0.92];
const random = randomFrom(62401);
const strandCount = 935;
const fringeCount = 640;
const fireRibbons = [
  { offset: 0.011, drift: 0.033, width: 0.2, color: "#df511c", alpha: 0.32 },
  { offset: 0.034, drift: 0.054, width: 0.13, color: "#fa7a24", alpha: 0.39 },
  { offset: 0.069, drift: 0.04, width: 0.075, color: "#ff9d32", alpha: 0.48 },
  { offset: -0.026, drift: 0.043, width: 0.097, color: "#ff9224", alpha: 0.66 },
  { offset: -0.008, drift: 0.023, width: 0.056, color: "#ffcb61", alpha: 0.88 },
  { offset: -0.014, drift: 0.025, width: 0.022, color: "#fff6cd", alpha: 0.9 },
  { offset: 0.014, drift: 0.036, width: 0.017, color: "#ffe49a", alpha: 0.77 },
] as const;
const ribbonFeather = [
  { width: 1, alpha: 0.17 },
  { width: 0.81, alpha: 0.23 },
  { width: 0.63, alpha: 0.31 },
  { width: 0.46, alpha: 0.69 },
] as const;
const filaments = Array.from({ length: strandCount }, (_, index) => ({
  phase: (index / strandCount) * TAU + (random() - 0.5) * 0.4,
  depth: random() - 0.5,
  turn: random() * TAU,
  offset: (random() - 0.42) * (0.061 + random() * 0.23),
  speed: 3.75 + random() * 1.95,
  span: 0.018 + Math.pow(random(), 2.3) * 0.39,
  flicker: 2.5 + random() * 7,
  heat: random(),
}));
const ejecta = Array.from({ length: 800 }, (_, index) => ({
  phase: (index / 800) * 1.31,
  period: 1.29 + random() * 0.44,
  life: index % 3 === 0 ? 0.24 + random() * 0.39 : 0.28 + random() * 1.04,
  tangent: 0.34 + Math.pow(random(), 1.3) * 2.48,
  spread: (random() - 0.11) * 1.55 + (index % 3 === 0 ? 1.18 : 0),
  span: 0.012 + Math.pow(random(), 2.6) * 0.068,
  bias: random() * TAU,
  heat: random(),
  radial: index % 3 === 0,
}));
const fringe = Array.from({ length: fringeCount }, (_, index) => {
  const heat = random();
  const inner = index % 4 === 0;
  return {
    phase: (index / fringeCount) * TAU + random() * 0.35,
    speed: 2.3 + random() * 3.2,
    drift: random() * TAU,
    offset: (inner ? -1 : 1) * (0.025 + random() ** 2 * (inner ? 0.11 : 0.185)),
    span: 0.005 + random() ** 3 * 0.16,
    heat: Math.floor(2 + heat * 4),
    ignition: random(),
  };
});
const bins = 5;

function ringRadius(frame: PortalFrame, time = frame.time): number {
  const expand = ease(ignition, 1.87, time);
  const variation = Math.sin((time - ignition) * 16) * 0.035 * (1 - ease(1.34, 2.32, time));
  return frame.radius * (mix(0.115, 1, expand) + variation);
}

function distance(theta: number, time: number, radius: number): number {
  return (
    radius *
    (1 +
      0.022 * Math.sin(theta * 3 + time * 2.1) +
      0.012 * Math.sin(theta * 7 - time * 5.5) +
      0.008 * Math.sin(theta * 13 + time * 4))
  );
}

function angularDistance(a: number, b: number): number {
  return Math.abs(((((a - b) % TAU) + 3 * Math.PI) % TAU) - Math.PI);
}

function leadingAngle(time: number): number {
  const age = Math.max(0, time - ignition);
  return 2.52 + age * 5.45 + 0.17 * Math.sin(age * 6.6);
}

function xy(
  path: Path2D,
  start: boolean,
  theta: number,
  time: number,
  radius: number,
  yScale: number,
) {
  const d = distance(theta, time, radius);
  const x = Math.cos(theta) * d + Math.sin(theta) * radius * 0.026;
  const y = Math.sin(theta) * d * yScale;
  if (start) path.moveTo(x, y);
  else path.lineTo(x, y);
}

export function portalWindow(frame: PortalFrame): Path2D {
  const path = new Path2D();
  const radius = ringRadius(frame) * 0.986;
  for (let i = 0; i < 124; i++) {
    xy(path, i === 0, (i / 124) * TAU, frame.time, radius, 0.968);
  }
  path.closePath();
  return path;
}

export function sparkLight(time: number): number {
  const formed = ease(ignition, 1.74, time);
  const peak = ease(0.9, 1.57, time) * (1 - ease(1.72, 2.66, time));
  return formed * (0.55 + 0.08 * Math.sin(time * 15)) + peak * 0.46;
}

function addFilaments(paths: Path2D[], frame: PortalFrame) {
  const { time } = frame;
  const radius = ringRadius(frame);
  const formed = ease(1.04, 1.92, time);
  const leading = leadingAngle(time);
  const ignite = ease(ignition, 0.95, time);
  for (const strand of filaments) {
    const theta = strand.phase + time * strand.speed;
    const focus = Math.max(
      1 - angularDistance(theta, leading) / mix(0.7, 2.7, formed),
      0.68 * (1 - angularDistance(theta, leading + 1.58) / mix(0.32, 1.69, formed)),
    );
    const pulse = 0.68 + 0.32 * Math.sin(theta * 2.18 + time * strand.flicker + strand.phase);
    const coil = Math.sin(theta * 3 - time * 13 + 1.1);
    const density = formed * (0.79 + coil * 0.13) + Math.max(0, focus) * 0.88;
    const alpha = ignite * density * pulse;
    if (alpha < 0.15 || density < strand.heat * 0.74) continue;
    const ripple = strand.depth * 0.077 * Math.sin(theta * 5.5 + time * 12 + strand.turn);
    const rad = radius * (1 + strand.offset + ripple);
    const arc = strand.span * mix(0.72, 1.35, Math.max(0, focus));
    const heat = Math.floor(
      clamp(5.45 - Math.abs(strand.offset) * 22 - strand.heat * 2.85 + focus, 0, 5),
    );
    const opacity = Math.floor(clamp(alpha * bins, 0, bins - 1));
    const path = paths[heat * bins + opacity]!;
    xy(path, true, theta - arc, time - 0.028, rad * (1 + strand.depth * 0.083), 0.968);
    xy(path, false, theta - arc * 0.5, time - 0.014, rad, 0.968);
    xy(path, false, theta, time, rad, 0.968);
  }
}

function drawFireRim(ctx: CanvasRenderingContext2D, frame: PortalFrame, fusion: number) {
  const { time } = frame;
  const radius = ringRadius(frame);
  for (const [index, ribbon] of fireRibbons.entries()) {
    const samples = Array.from({ length: 129 }, (_, i) => {
      const theta = i * (TAU / 128);
      const flow = theta * 3 - time * 7.2 + index * 1.67;
      const folds = 0.44 * Math.sin(flow) + 0.34 * Math.sin(flow * 2 + time * 2.1);
      const flame = Math.sin(theta * 11 - time * 11 + index) * 0.12;
      const width = radius * ribbon.width * (0.45 + folds * 0.28 + flame);
      const offset = ribbon.offset + ribbon.drift * folds;
      const d = distance(theta, time, radius) + radius * offset;
      return { dx: Math.cos(theta), dy: Math.sin(theta), d, width };
    });
    ctx.fillStyle = ribbon.color;
    for (const feather of ribbonFeather) {
      const band = new Path2D();
      for (let edge = 0; edge < 2; edge++) {
        for (let i = 0; i <= 128; i++) {
          const { dx, dy, d, width } = samples[edge === 0 ? i : 128 - i]!;
          const dist = d + width * feather.width * (1 - edge * 2);
          const x = dx * dist + dy * radius * 0.026;
          const y = dy * dist * 0.968;
          if (i === 0 && edge === 0) band.moveTo(x, y);
          else band.lineTo(x, y);
        }
      }
      band.closePath();
      ctx.globalAlpha = ribbon.alpha * feather.alpha * fusion;
      ctx.fill(band);
    }
  }
}

function drawFireGrains(ctx: CanvasRenderingContext2D, frame: PortalFrame, fusion: number) {
  const { time } = frame;
  const radius = ringRadius(frame);
  const ember = new Path2D();
  const hot = new Path2D();
  for (const strand of fringe) {
    if (strand.ignition > fusion) continue;
    const theta = strand.phase + time * (strand.speed + 0.94);
    const twinkle = Math.sin(theta * 5.2 - strand.drift + time * 14);
    if (twinkle < -0.2) continue;
    const offset = strand.offset + 0.027 * Math.sin(theta * 3 - time * 7.2);
    const path = twinkle > 0.65 ? hot : ember;
    const rad = radius * (1 + offset);
    xy(path, true, theta - 0.002 - strand.ignition * 0.005, time, rad, 0.968);
    xy(path, false, theta, time, rad, 0.968);
  }
  ctx.lineWidth = clamp(radius * 0.008, 0.7, 1.9);
  ctx.strokeStyle = "#ffb63d";
  ctx.globalAlpha = fusion * 0.73;
  ctx.stroke(ember);
  ctx.strokeStyle = "#fff5cb";
  ctx.globalAlpha = fusion * 0.87;
  ctx.stroke(hot);
}

function addFringe(paths: Path2D[], frame: PortalFrame, fusion: number) {
  const { time } = frame;
  const radius = ringRadius(frame);
  for (const ember of fringe) {
    if (ember.ignition > fusion) continue;
    const theta = ember.phase + time * ember.speed;
    const wave = Math.sin(theta * 4 - time * 9.6 + ember.drift);
    const shimmer = 0.52 + 0.46 * Math.sin(theta * 7 + time * 11 + ember.drift);
    const intensity = fusion * shimmer;
    if (intensity < 0.13) continue;
    const rad = radius * (1 + ember.offset * (0.87 + wave * 0.26));
    const opacity = Math.floor(clamp(intensity * bins, 0, bins - 1));
    const path = paths[ember.heat * bins + opacity]!;
    xy(
      path,
      true,
      theta - ember.span,
      time - 0.018,
      rad + wave * ember.span * radius * 0.33,
      0.968,
    );
    xy(path, false, theta - ember.span * 0.5, time - 0.009, rad, 0.968);
    xy(path, false, theta, time, rad, 0.968);
  }
}

function addEjecta(paths: Path2D[], frame: PortalFrame, floor: number) {
  const { time, radius } = frame;
  for (const spark of ejecta) {
    // Reconstruct each ballistic spark from its birth. Sampling its trajectory
    // twice makes a shutter trail and keeps pauses/resizes independent of FPS.
    const age = (time + spark.phase) % spark.period;
    const born = time - age;
    if (born < ignition + 0.035 || age >= spark.life) continue;
    const formed = ease(1.28, 2.29, born);
    const theta =
      (spark.radial ? born * 2.07 + leadingAngle(born) : leadingAngle(born)) -
      spark.bias * mix(0.085, 1.18, formed) +
      formed * 0.32 * Math.sin(born * 2.37 + spark.bias * 5);
    const dx = Math.cos(theta);
    const dy = Math.sin(theta);
    const emissionRadius = distance(theta, born, ringRadius(frame, born));
    const vx = -dy * spark.tangent + dx * spark.spread;
    const vy = dx * spark.tangent * 0.88 + dy * spark.spread;
    const headX = dx * emissionRadius;
    const headY = dy * (emissionRadius * 0.968);
    const tail = Math.max(0, age - spark.span * (1.13 - 0.4 * age));
    const travel = ((1 - Math.exp(-age * 1.65)) * radius) / 1.65;
    const tailTravel = ((1 - Math.exp(-tail * 1.65)) * radius) / 1.65;
    const gravity = radius * 1.17;
    const remaining = 1 - age / spark.life;
    const heat = Math.floor(clamp(remaining * 5.5 - spark.heat * 1.3, 0, 5));
    const y = headY + vy * travel + gravity * age * age;
    const above = y + frame.y;
    const foot = 1 - ease(frame.height - 10, floor - 3, above);
    const strength = remaining * Math.sqrt(foot);
    if (strength < 0.07) continue;
    const opacity = Math.floor(clamp(strength * bins, 0, bins - 1));
    const path = paths[heat * bins + opacity]!;
    path.moveTo(headX + vx * tailTravel, headY + vy * tailTravel + gravity * tail * tail);
    const midway = (age + tail) / 2;
    const middleTravel = (travel + tailTravel) / 2;
    path.quadraticCurveTo(
      headX + vx * middleTravel,
      headY + vy * middleTravel + gravity * midway * midway,
      headX + vx * travel,
      y,
    );
  }
}

export function createSparkGlow(): HTMLCanvasElement {
  const tile = document.createElement("canvas");
  tile.width = tile.height = 64;
  const ctx = tile.getContext("2d");
  if (!ctx) return tile;
  const light = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
  light.addColorStop(0, "#fff7c9d6");
  light.addColorStop(0.16, "#ffbc58a6");
  light.addColorStop(0.46, "#ff771e53");
  light.addColorStop(1, "#ff551500");
  ctx.fillStyle = light;
  ctx.fillRect(0, 0, 64, 64);
  return tile;
}

function drawHotSpots(ctx: CanvasRenderingContext2D, frame: PortalFrame, glow: HTMLCanvasElement) {
  const { time } = frame;
  const radius = ringRadius(frame);
  const build = ease(ignition, 0.98, time);
  const formed = ease(1.18, 1.9, time);
  const lead = leadingAngle(time);
  for (let index = 0; index < 31; index++) {
    const streak = filaments[index * 22]!;
    const angle = streak.phase * formed + lead - 0.46 * streak.heat + time * streak.depth;
    const dist = distance(angle, time, radius);
    const x = Math.cos(angle) * dist + Math.sin(angle) * radius * 0.026;
    const y = Math.sin(angle) * dist * 0.968;
    const pulse = Math.pow(0.5 + 0.5 * Math.sin(time * streak.flicker * 1.8 + streak.turn), 3);
    const size = frame.radius * (0.12 + pulse * 0.17);
    ctx.globalAlpha = pulse * build * (1 - formed * 0.35) * 0.81;
    ctx.drawImage(glow, x - size * 0.5, y - size * 0.5, size, size);
  }
}

export function drawSparks(
  ctx: CanvasRenderingContext2D,
  frame: PortalFrame,
  glow: HTMLCanvasElement,
  floor: number,
) {
  if (frame.time < ignition) return;
  const paths = Array.from({ length: palette.length * bins }, () => new Path2D());
  const fusion = ease(1.28, 2.13, frame.time);
  addFilaments(paths, frame);
  addEjecta(paths, frame, floor);
  if (fusion > 0) addFringe(paths, frame, fusion);
  ctx.save();
  ctx.translate(frame.x, frame.y);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  const scale = clamp(frame.radius / 120, 0.66, 1.16);

  if (fusion > 0) drawFireRim(ctx, frame, fusion);
  drawHotSpots(ctx, frame, glow);

  // The low-opacity wide strokes are driven by the same filaments as the
  // bright strokes, so the glow travels with each arc.
  for (let heat = 2; heat < palette.length; heat++) {
    ctx.strokeStyle = heat < 4 ? "#e26125" : "#ff9a36";
    ctx.lineWidth = 6.4 * scale;
    for (let opacity = 1; opacity < bins; opacity++) {
      ctx.globalAlpha = 0.034 + opacity * 0.022;
      ctx.stroke(paths[heat * bins + opacity]!);
    }
  }

  for (let heat = 0; heat < palette.length; heat++) {
    ctx.strokeStyle = palette[heat]!;
    ctx.lineWidth = strokes[heat]! * scale;
    for (let opacity = 0; opacity < bins; opacity++) {
      ctx.globalAlpha = 0.085 + opacity * 0.193;
      ctx.stroke(paths[heat * bins + opacity]!);
    }
  }
  if (fusion > 0) drawFireGrains(ctx, frame, fusion);
  ctx.restore();
}
