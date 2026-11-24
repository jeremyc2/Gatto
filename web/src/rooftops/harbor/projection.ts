import { clamp, rgbCss, type ArtPath, type Point2, type Point3, type Rect4 } from "../types";

export const HARBOR_WIDTH = 900;
export const HARBOR_HEIGHT = 560;
export const HORIZON = 0.755;
export const WATER = 0x111e32;
export const TRIM = 0x938095;
export const LIGHT = 0xf5be87;

export const FERRY_DEPTH = -216;
export const SAILBOAT_DEPTH = -238;
export const TUG_DEPTH = 8;
export const WAKE_STREAKS = 9;
export const FOREGROUND_BOAT_SPEED = 14;
export const TUG_TURN_DISTANCE = 12;
export const TUG_PHASE = 1.32;
export const HARBOR_YAW = 0.64;
export const VIEW_DEPTH = 0.72;
export const VIEW_RECESSION = 0.13;
export const MOON_CENTER: Point2 = [832, 64];
export const MOON_DIAMETER = 64;

export interface Aim {
  x: number;
  y: number;
}

export function defaultAim(): Aim {
  return { x: 0, y: 0 };
}

export function aimFromPointer(px: number, py: number, width: number, height: number): Aim {
  const axis = (pos: number, extent: number) => {
    const e = Math.max(extent, 1);
    return -clamp((2 * pos) / e - 1, -1, 1);
  };
  return { x: axis(px, width), y: axis(py, height) };
}

export function followAim(current: Aim, target: Aim, elapsed: number): Aim {
  const amount = 1 - Math.exp(-12 * elapsed);
  const step = (c: number, t: number) => (Math.abs(c - t) < 0.005 ? t : c + (t - c) * amount);
  return { x: step(current.x, target.x), y: step(current.y, target.y) };
}

export function mix3(a: Point3, b: Point3, t: number): Point3 {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

export function blendInk(from: number, to: number, amount: number): number {
  const fr = (from >> 16) & 255;
  const fg = (from >> 8) & 255;
  const fb = from & 255;
  const tr = (to >> 16) & 255;
  const tg = (to >> 8) & 255;
  const tb = to & 255;
  const r = Math.round(fr + (tr - fr) * amount);
  const g = Math.round(fg + (tg - fg) * amount);
  const b = Math.round(fb + (tb - fb) * amount);
  return ((r & 255) << 16) | ((g & 255) << 8) | (b & 255);
}

export function shadeInk(ink: number, factor: number): number {
  const r = Math.min(255, Math.floor(((ink >> 16) & 255) * factor));
  const g = Math.min(255, Math.floor(((ink >> 8) & 255) * factor));
  const b = Math.min(255, Math.floor((ink & 255) * factor));
  return (r << 16) | (g << 8) | b;
}

function dot(a: Point3, b: Point3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

export function pushShape(paths: ArtPath[], points: Point2[], ink: number, opacity: number): void {
  paths.push({ points, ink, opacity });
}

export function pushQuad(paths: ArtPath[], rect: Rect4, ink: number, opacity: number): void {
  const [x, y, w, h] = rect;
  paths.push({
    points: [
      [x, y],
      [x + w, y],
      [x + w, y + h],
      [x, y + h],
    ],
    ink,
    opacity,
  });
}

export function pushStroke(
  paths: ArtPath[],
  a: Point2,
  b: Point2,
  width: number,
  ink: number,
  opacity: number,
): void {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const scale = (width * 0.5) / Math.max(Math.hypot(dx, dy), 0.001);
  const ox = -dy * scale;
  const oy = dx * scale;
  paths.push({
    points: [
      [a[0] + ox, a[1] + oy],
      [b[0] + ox, b[1] + oy],
      [b[0] - ox, b[1] - oy],
      [a[0] - ox, a[1] - oy],
    ],
    ink,
    opacity,
  });
}

export function pushOval(
  paths: ArtPath[],
  center: Point2,
  radii: Point2,
  ink: number,
  opacity: number,
): void {
  const points: Point2[] = [];
  for (let i = 0; i < 16; i++) {
    const angle = (i * Math.PI * 2) / 16;
    points.push([center[0] + radii[0] * Math.cos(angle), center[1] + radii[1] * Math.sin(angle)]);
  }
  paths.push({ points, ink, opacity });
}

export class Camera {
  readonly eye: Point3;
  readonly axes: [Point3, Point3, Point3];
  readonly center: Point2;
  readonly clip: Rect4;
  private readonly planeList: readonly (Point3 | null)[];

  constructor(eye: Point3, axes: [Point3, Point3, Point3], center: Point2, clip: Rect4) {
    this.eye = eye;
    this.axes = axes;
    this.center = center;
    this.clip = clip;
    const [cx, cy] = center;
    const [left, top, right, bottom] = clip;
    this.planeList = [
      null,
      [1, 0, cx - left + 16],
      [-1, 0, right - cx + 16],
      [0, 1, cy - top + 16],
      [0, -1, bottom - cy + 16],
    ];
  }

  static atMenu(view: View): Camera {
    const { x, y } = view.aim;
    const yaw = HARBOR_YAW + x * 0.07;
    const recession = VIEW_RECESSION + x * 0.04;
    const tilt = 0.25 + y * 0.035;
    const lens = 0.7 / view.width;
    const z = (-1 / lens - recession * x * 2.8) / (VIEW_DEPTH + recession * yaw);
    const dx = -yaw * z - x * 2.8;
    const forward: Point3 = [-recession, 0, VIEW_DEPTH];
    const vertical: Point3 = [0.012 / lens, 1 / lens, -tilt / lens];
    const offset = view.horizon - HARBOR_HEIGHT * 0.5;
    return new Camera(
      [view.width * 0.5 + dx, view.horizon + z * tilt - dx * 0.012 - y * 2, z],
      [
        [1 / lens, 0, yaw / lens],
        [
          vertical[0] + offset * forward[0],
          vertical[1] + offset * forward[1],
          vertical[2] + offset * forward[2],
        ],
        forward,
      ],
      [view.width * 0.5, HARBOR_HEIGHT * 0.5],
      [0, 0, HARBOR_WIDTH, HARBOR_HEIGHT],
    );
  }

  dolly(eye: Point3, pitch: number): Camera {
    const sine = Math.sin(pitch);
    const cosine = Math.cos(pitch);
    const focal = 1550;
    const [ax, y, z] = this.axes;
    const nextY: Point3 = [
      y[0] * cosine - z[0] * focal * sine,
      y[1] * cosine - z[1] * focal * sine,
      y[2] * cosine - z[2] * focal * sine,
    ];
    const nextZ: Point3 = [
      z[0] * cosine + (y[0] * sine) / focal,
      z[1] * cosine + (y[1] * sine) / focal,
      z[2] * cosine + (y[2] * sine) / focal,
    ];
    return new Camera(eye, [ax, nextY, nextZ], this.center, this.clip);
  }

  withClip(clip: Rect4): Camera {
    return new Camera(this.eye, this.axes, this.center, clip);
  }

  withEye(eye: Point3): Camera {
    return new Camera(eye, this.axes, this.center, this.clip);
  }

  transform(point: Point3): Point3 {
    const dx = point[0] - this.eye[0];
    const dy = point[1] - this.eye[1];
    const dz = point[2] - this.eye[2];
    const [a0, a1, a2] = this.axes;
    return [
      a0[0] * dx + a0[1] * dy + a0[2] * dz,
      a1[0] * dx + a1[1] * dy + a1[2] * dz,
      a2[0] * dx + a2[1] * dy + a2[2] * dz,
    ];
  }

  screen(p: Point3): Point2 {
    return [this.center[0] + p[0] / p[2], this.center[1] + p[1] / p[2]];
  }

  planes(): readonly (Point3 | null)[] {
    return this.planeList;
  }

  face(inputPoints: readonly Point3[]): Point2[] {
    let points: Point3[] = [];
    let outsideAll = 0b11111;
    let outsideAny = 0;
    const planes = this.planeList;

    for (let pi = 0; pi < inputPoints.length; pi++) {
      const p = this.transform(inputPoints[pi]!);
      points.push(p);
      let mask = 0;
      if (p[2] - 4 < 0) mask |= 1;
      for (let i = 1; i < 5; i++) {
        if (dot(planes[i]!, p) < 0) mask |= 1 << i;
      }
      outsideAll &= mask;
      outsideAny |= mask;
    }

    if (outsideAll !== 0) return [];

    if (outsideAny !== 0) {
      for (let i = 0; i < 5; i++) {
        if ((outsideAny & (1 << i)) === 0) continue;
        if (points.length === 0) break;
        const plane = planes[i]!;
        const clipped: Point3[] = [];
        let a = points[points.length - 1]!;
        let da = plane === null ? a[2] - 4 : dot(plane, a);
        for (let j = 0; j < points.length; j++) {
          const b = points[j]!;
          const db = plane === null ? b[2] - 4 : dot(plane, b);
          if (da >= 0 !== db >= 0) {
            clipped.push(mix3(a, b, da / (da - db)));
          }
          if (db >= 0) {
            clipped.push(b);
          }
          a = b;
          da = db;
        }
        points = clipped;
      }
    }

    const out: Point2[] = [];
    for (let i = 0; i < points.length; i++) {
      out.push(this.screen(points[i]!));
    }
    return out;
  }

  point(p: Point3): Point2 | null {
    const at = this.transform(p);
    return at[2] >= 4 ? this.screen(at) : null;
  }
}

export class View {
  readonly width: number;
  readonly horizon: number;
  readonly aim: Aim;
  readonly camera: Camera | null;

  constructor(width: number, horizon: number, aim: Aim, camera: Camera | null = null) {
    this.width = width;
    this.horizon = horizon;
    this.aim = aim;
    this.camera = camera;
  }

  withCamera(camera: Camera | null): View {
    return new View(this.width, this.horizon, this.aim, camera);
  }

  neutral(): View {
    return new View(this.width, this.horizon, defaultAim(), null);
  }

  magnification(x: number, z: number): number {
    const dx = x - this.width * 0.5;
    const distance = z * VIEW_DEPTH - dx * (VIEW_RECESSION + 0.04 * this.aim.x);
    return 1 / (1 + distance * (0.7 / this.width));
  }

  project(x: number, z: number, y: number): Point2 {
    const dx = x - this.width * 0.5;
    const zoom = this.magnification(x, z);
    const yaw = HARBOR_YAW + this.aim.x * 0.07;
    const tilt = 0.25 + this.aim.y * 0.035;
    return [
      this.width * 0.5 + (dx + yaw * z + this.aim.x * 2.8) * zoom,
      this.horizon + (y - this.horizon - z * tilt + dx * 0.012 + this.aim.y * 2) * zoom,
    ];
  }

  xAtDepth(screenX: number, depth: number): number {
    const center = this.width * 0.5;
    const dx = screenX - center;
    const lens = 0.7 / this.width;
    const yaw = HARBOR_YAW + this.aim.x * 0.07;
    const recession = VIEW_RECESSION + this.aim.x * 0.04;
    return (
      center +
      (dx * (1 + depth * VIEW_DEPTH * lens) - yaw * depth - this.aim.x * 2.8) /
        (1 + dx * recession * lens)
    );
  }

  rear(p: Point3, depth: number): Point3 {
    const yaw = HARBOR_YAW + this.aim.x * 0.07;
    return [p[0] - yaw * depth, p[1], p[2] + depth];
  }

  deck(): number {
    return this.horizon - 53;
  }

  waterline(): number {
    return this.horizon + 29;
  }

  onWater(x: number, depth: number): Point2 {
    return this.project(x, depth, this.waterline());
  }

  groundBillboard(screenX: number, depth: number, grade: number): [Point2, number] {
    const n = this.neutral();
    const x = n.xAtDepth(screenX, depth);
    const zoom = this.magnification(x, depth) / n.magnification(x, depth);
    return [this.project(x, depth, grade), zoom];
  }

  billboard(x: number, z: number, y: number): [Point2, number] {
    const n = this.neutral();
    const [atX, atY] = this.project(x, z, y);
    const [restX, restY] = n.project(x, z, y);
    return [
      [x + (atX - restX), y + (atY - restY)],
      this.magnification(x, z) / n.magnification(x, z),
    ];
  }

  polygon(points: readonly Point3[]): Point2[] {
    if (this.camera) return this.camera.face(points);
    const out: Point2[] = [];
    for (let i = 0; i < points.length; i++) {
      const p = points[i]!;
      out.push(this.project(p[0], p[2], p[1]));
    }
    return out;
  }

  face(paths: ArtPath[], points: readonly Point3[], ink: number, opacity: number): void {
    const poly = this.polygon(points);
    if (poly.length >= 3) {
      pushShape(paths, poly, ink, opacity);
    }
  }

  line(paths: ArtPath[], a: Point3, b: Point3, width: number, ink: number, opacity: number): void {
    if (this.camera) {
      let ta = this.camera.transform(a);
      let tb = this.camera.transform(b);
      const planes = this.camera.planes();
      for (let i = 0; i < 5; i++) {
        const plane = planes[i]!;
        const da = plane === null ? ta[2] - 4 : dot(plane, ta);
        const db = plane === null ? tb[2] - 4 : dot(plane, tb);
        if (da < 0 && db < 0) return;
        if (da < 0) ta = mix3(ta, tb, da / (da - db));
        else if (db < 0) tb = mix3(ta, tb, da / (da - db));
      }
      const zoom = clamp(690 / Math.min(ta[2], tb[2]), 0.85, 6);
      pushStroke(paths, this.camera.screen(ta), this.camera.screen(tb), width * zoom, ink, opacity);
    } else {
      pushStroke(
        paths,
        this.project(a[0], a[2], a[1]),
        this.project(b[0], b[2], b[1]),
        width,
        ink,
        opacity,
      );
    }
  }

  offset(p: Point3, dx: number, dy: number): Point3 {
    if (this.camera) {
      return [p[0] + dx, p[1] + dy, p[2]];
    }
    const [sx, sy] = this.project(p[0], p[2], p[1]);
    return this.lift([sx + dx, sy + dy], p[2]);
  }

  detail(paths: ArtPath[], at: Point3, rect: Rect4, ink: number, opacity: number): void {
    const [x, y, w, h] = rect;
    this.face(
      paths,
      [
        this.offset(at, x, y),
        this.offset(at, x + w, y),
        this.offset(at, x + w, y + h),
        this.offset(at, x, y + h),
      ],
      ink,
      opacity,
    );
  }

  lift(screen: Point2, z: number): Point3 {
    const x = this.xAtDepth(screen[0], z);
    const y =
      this.horizon + (screen[1] - this.project(x, z, this.horizon)[1]) / this.magnification(x, z);
    return [x, y, z];
  }
}

export interface CompiledLayer {
  path: Path2D;
  ink: number;
  opacity: number;
  solid: boolean;
}

/**
 * Compile consecutive same-(ink, opacity) polygons into Path2D batches,
 * ensuring consistent clockwise winding on solid shapes so nonzero fill
 * always unions overlapping contours cleanly.
 */
export function compileArtMesh(shapes: readonly ArtPath[]): CompiledLayer[] {
  const layers: CompiledLayer[] = [];
  let i = 0;
  while (i < shapes.length) {
    const first = shapes[i]!;
    const ink = first.ink;
    const opacity = first.opacity;
    const solid = opacity === 1;
    const path = new Path2D();
    let hasPolys = false;

    while (i < shapes.length) {
      const s = shapes[i]!;
      if (s.ink !== ink || s.opacity !== opacity) break;
      i += 1;
      const pts = s.points;
      if (pts.length < 3) continue;
      hasPolys = true;

      let reversed = false;
      if (solid) {
        let area2 = 0;
        for (let k = 0; k < pts.length; k++) {
          const a = pts[k]!;
          const b = pts[(k + 1) % pts.length]!;
          area2 += (a[0] - b[0]) * (a[1] + b[1]);
        }
        reversed = area2 < 0;
      }

      if (reversed) {
        const start = pts[pts.length - 1]!;
        path.moveTo(start[0], start[1]);
        for (let k = pts.length - 2; k >= 0; k--) {
          const p = pts[k]!;
          path.lineTo(p[0], p[1]);
        }
      } else {
        const start = pts[0]!;
        path.moveTo(start[0], start[1]);
        for (let k = 1; k < pts.length; k++) {
          const p = pts[k]!;
          path.lineTo(p[0], p[1]);
        }
      }
      path.closePath();
    }

    if (hasPolys) {
      layers.push({ path, ink, opacity, solid });
    }
  }
  return layers;
}

export function drawCompiledMesh(
  ctx: CanvasRenderingContext2D,
  layers: readonly CompiledLayer[],
  scale: number,
): void {
  ctx.save();
  ctx.scale(scale, scale);
  let lastInk = -1;
  let lastAlpha = -1;
  for (let i = 0; i < layers.length; i++) {
    const layer = layers[i]!;
    if (layer.ink !== lastInk) {
      ctx.fillStyle = rgbCss(layer.ink);
      lastInk = layer.ink;
    }
    if (layer.opacity !== lastAlpha) {
      ctx.globalAlpha = layer.opacity;
      lastAlpha = layer.opacity;
    }
    ctx.fill(layer.path, layer.solid ? "nonzero" : "evenodd");
  }
  ctx.restore();
  ctx.globalAlpha = 1;
}

/** Fast batched draw directly from ArtPath[] without caching Path2D objects. */
export function drawHarborPaths(
  ctx: CanvasRenderingContext2D,
  shapes: readonly ArtPath[],
  scale: number,
): void {
  let i = 0;
  while (i < shapes.length) {
    const first = shapes[i]!;
    const ink = first.ink;
    const opacity = first.opacity;
    if (opacity <= 0) {
      i += 1;
      continue;
    }
    const solid = opacity === 1;
    ctx.fillStyle = rgbCss(ink);
    ctx.globalAlpha = opacity;
    ctx.beginPath();
    let count = 0;
    while (i < shapes.length) {
      const s = shapes[i]!;
      if (s.ink !== ink || s.opacity !== opacity) break;
      i += 1;
      const pts = s.points;
      if (pts.length < 3) continue;
      count += 1;
      let reversed = false;
      if (solid) {
        let area2 = 0;
        for (let k = 0; k < pts.length; k++) {
          const a = pts[k]!;
          const b = pts[(k + 1) % pts.length]!;
          area2 += (a[0] - b[0]) * (a[1] + b[1]);
        }
        reversed = area2 < 0;
      }
      if (reversed) {
        const start = pts[pts.length - 1]!;
        ctx.moveTo(start[0] * scale, start[1] * scale);
        for (let k = pts.length - 2; k >= 0; k--) {
          const p = pts[k]!;
          ctx.lineTo(p[0] * scale, p[1] * scale);
        }
      } else {
        const start = pts[0]!;
        ctx.moveTo(start[0] * scale, start[1] * scale);
        for (let k = 1; k < pts.length; k++) {
          const p = pts[k]!;
          ctx.lineTo(p[0] * scale, p[1] * scale);
        }
      }
      ctx.closePath();
    }
    if (count > 0) {
      ctx.fill(solid ? "nonzero" : "evenodd");
    }
  }
  ctx.globalAlpha = 1;
}
