// Renders the flying portal cat from one rigid model so every angle keeps the
// same anatomy, harness, thrusters and exhaust. The output is native-resolution
// pixel art in the approved palette; Aseprite masters are assembled from it.
import { deflateSync } from "node:zlib";

type Vec = [number, number, number];
type Basis = [Vec, Vec, Vec]; // forward, up, right (world-space columns)

const add = (a: Vec, b: Vec): Vec => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: Vec, b: Vec): Vec => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const mul = (a: Vec, s: number): Vec => [a[0] * s, a[1] * s, a[2] * s];
const dot = (a: Vec, b: Vec) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec, b: Vec): Vec => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const norm = (a: Vec): Vec => mul(a, 1 / Math.hypot(a[0], a[1], a[2]));
const clamp = (v: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, v));
const mix = (a: number, b: number, t: number) => a + (b - a) * t;
const smooth = (t: number) => t * t * (3 - 2 * t);

// World: x right, y up, z toward the viewer (right-handed).
function basis(forward: Vec, upHint: Vec): Basis {
  const f = norm(forward);
  const u = norm(sub(upHint, mul(f, dot(upHint, f))));
  return [f, u, cross(f, u)];
}

type Quat = [number, number, number, number];
function toQuat([f, u, r]: Basis): Quat {
  // Rotation matrix columns are the local axes.
  const m00 = f[0],
    m01 = u[0],
    m02 = r[0];
  const m10 = f[1],
    m11 = u[1],
    m12 = r[1];
  const m20 = f[2],
    m21 = u[2],
    m22 = r[2];
  const trace = m00 + m11 + m22;
  let w, x, y, z;
  if (trace > 0) {
    const s = Math.sqrt(trace + 1) * 2;
    w = s / 4;
    x = (m21 - m12) / s;
    y = (m02 - m20) / s;
    z = (m10 - m01) / s;
  } else if (m00 > m11 && m00 > m22) {
    const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
    w = (m21 - m12) / s;
    x = s / 4;
    y = (m01 + m10) / s;
    z = (m02 + m20) / s;
  } else if (m11 > m22) {
    const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
    w = (m02 - m20) / s;
    x = (m01 + m10) / s;
    y = s / 4;
    z = (m12 + m21) / s;
  } else {
    const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
    w = (m10 - m01) / s;
    x = (m02 + m20) / s;
    y = (m12 + m21) / s;
    z = s / 4;
  }
  return [w, x, y, z];
}

function fromQuat([w, x, y, z]: Quat): Basis {
  const col = (vx: number, vy: number, vz: number): Vec => [vx, vy, vz];
  return [
    col(1 - 2 * (y * y + z * z), 2 * (x * y + w * z), 2 * (x * z - w * y)),
    col(2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x)),
    col(2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)),
  ];
}

function slerp(a: Quat, b: Quat, t: number): Quat {
  let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
  const target = [...b] as Quat;
  if (d < 0) {
    d = -d;
    for (let i = 0; i < 4; i++) target[i] = -target[i]!;
  }
  if (d > 0.9995) {
    const q = a.map((v, i) => mix(v, target[i]!, t)) as Quat;
    const l = Math.hypot(...q);
    return q.map((v) => v / l) as Quat;
  }
  const theta = Math.acos(d);
  const sa = Math.sin((1 - t) * theta) / Math.sin(theta);
  const sb = Math.sin(t * theta) / Math.sin(theta);
  return a.map((v, i) => v * sa + target[i]! * sb) as Quat;
}

const turnBasis = (b: Basis, t: number, to: Basis) => fromQuat(slerp(toQuat(b), toQuat(to), t));

// --- noise -----------------------------------------------------------------

function hash(x: number, y: number, z: number) {
  let h = (x * 374761393 + y * 668265263 + z * 2147483647) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
}

function noise(x: number, y: number, z: number) {
  const xi = Math.floor(x),
    yi = Math.floor(y),
    zi = Math.floor(z);
  const xf = smooth(x - xi),
    yf = smooth(y - yi),
    zf = smooth(z - zi);
  const c = (dx: number, dy: number, dz: number) => hash(xi + dx, yi + dy, zi + dz);
  const x00 = mix(c(0, 0, 0), c(1, 0, 0), xf);
  const x10 = mix(c(0, 1, 0), c(1, 1, 0), xf);
  const x01 = mix(c(0, 0, 1), c(1, 0, 1), xf);
  const x11 = mix(c(0, 1, 1), c(1, 1, 1), xf);
  return mix(mix(x00, x10, yf), mix(x01, x11, yf), zf);
}

const fbm = (x: number, y: number, z: number) =>
  noise(x, y, z) * 0.6 +
  noise(x * 2.1 + 7, y * 2.1, z * 2.1) * 0.3 +
  noise(x * 4.3, y * 4.3 + 3, z * 4.3) * 0.1;

// --- distance primitives ---------------------------------------------------

function sdEllipsoid(x: number, y: number, z: number, rx: number, ry: number, rz: number) {
  const k0 = Math.hypot(x / rx, y / ry, z / rz);
  const k1 = Math.hypot(x / (rx * rx), y / (ry * ry), z / (rz * rz));
  return k1 === 0 ? -Math.min(rx, ry, rz) : (k0 * (k0 - 1)) / k1;
}

function sdRoundCone(p: Vec, a: Vec, b: Vec, r1: number, r2: number) {
  const ba = sub(b, a);
  const l2 = dot(ba, ba);
  const rr = r1 - r2;
  const a2 = l2 - rr * rr;
  const il2 = 1 / l2;
  const pa = sub(p, a);
  const y = dot(pa, ba);
  const z = y - l2;
  const q = sub(mul(pa, l2), mul(ba, y));
  const x2 = dot(q, q);
  const y2 = y * y * l2;
  const z2 = z * z * l2;
  const k = Math.sign(rr) * rr * rr * x2;
  if (Math.sign(z) * a2 * z2 > k) return Math.sqrt(x2 + z2) * il2 - r2;
  if (Math.sign(y) * a2 * y2 < k) return Math.sqrt(x2 + y2) * il2 - r1;
  return (Math.sqrt(x2 * a2 * il2) + y * rr) * il2 - r1;
}

function smin(a: number, b: number, k: number) {
  const h = Math.max(k - Math.abs(a - b), 0) / k;
  return Math.min(a, b) - h * h * k * 0.25;
}

function sdRoundBox(
  x: number,
  y: number,
  z: number,
  bx: number,
  by: number,
  bz: number,
  r: number,
) {
  const qx = Math.abs(x) - bx + r,
    qy = Math.abs(y) - by + r,
    qz = Math.abs(z) - bz + r;
  return (
    Math.hypot(Math.max(qx, 0), Math.max(qy, 0), Math.max(qz, 0)) +
    Math.min(Math.max(qx, qy, qz), 0) -
    r
  );
}

// --- model (body-local: x forward, y dorsal, z cat's right) ----------------

const enum M {
  None,
  Fur,
  Nose,
  Metal,
  Nozzle,
  Strap,
  Buckle,
  Plate,
}

interface Limb {
  points: Vec[];
  radii: number[];
}

// One foreleg and one hind leg per side, mirrored, so both sides always match.
const foreleg: Limb = {
  points: [
    [20, -6, 10.5],
    [33, -20, 9.5],
    [59, -18, 7.4],
    [68, -16, 6.8],
    [75, -14.5, 6.4],
  ],
  radii: [11, 8.8, 7, 7.8, 6.6],
};
const hindleg: Limb = {
  points: [
    [-20, -4, 11.5],
    [-37, -15, 10.5],
    [-59, -18, 8.6],
    [-71, -17, 8],
    [-79, -15, 7.8],
  ],
  radii: [14, 9.4, 6.4, 7.2, 6.2],
};
const tail: Limb = {
  points: [
    [-36, 8, 0],
    [-58, 4, 0],
    [-82, 0, 0],
    [-104, -3, 0],
    [-114, -4, 0],
  ],
  radii: [7, 6, 5.2, 4.6, 4.3],
};

const neckBase: Vec = [30, 13, 0];
// Head-local (x facing direction, y up, z the cat's right).
const neckTop: Vec = [-6, -31, 0];
const nozzleExit = -35;
const thrusterY = 36;
const thrusterZ = 8.6;
const thrusterRadius = 8;

function limbDistance(p: Vec, limb: Limb, k: number) {
  let d = Infinity;
  for (let i = 0; i < limb.points.length - 1; i++) {
    const s = sdRoundCone(
      p,
      limb.points[i]!,
      limb.points[i + 1]!,
      limb.radii[i]!,
      limb.radii[i + 1]!,
    );
    d = i === 0 ? s : smin(d, s, k);
  }
  return d;
}

function torso(x: number, y: number, z: number) {
  const chest = sdEllipsoid(x - 13, y - 1, z, 27, 26, 24);
  const rump = sdEllipsoid(x + 15, y, z, 27, 24, 23);
  return smin(chest, rump, 14);
}

// --- frame state -----------------------------------------------------------

interface Pose {
  body: Basis;
  head: Basis;
  headCenter: Vec; // world, relative to body origin
  scale: number;
  phase: number; // exhaust flicker phase (radians)
}

let pose: Pose;
let neckTopBody: Vec = [0, 0, 0];
let headInBody: Basis;
let headCenterBody: Vec = [0, 0, 0];

function toBody(p: Vec): Vec {
  const [f, u, r] = pose.body;
  return [dot(p, f), dot(p, u), dot(p, r)];
}

function headLocalFromBody(p: Vec): Vec {
  const d = sub(p, headCenterBody);
  const [f, u, r] = headInBody;
  return [dot(d, f), dot(d, u), dot(d, r)];
}

function setPose(next: Pose) {
  pose = next;
  headCenterBody = toBody(next.headCenter);
  headInBody = next.head.map((axis) => toBody(axis)) as Basis;
  const [f, u, r] = headInBody;
  neckTopBody = add(
    headCenterBody,
    add(add(mul(f, neckTop[0]), mul(u, neckTop[1])), mul(r, neckTop[2])),
  );
}

// Eyes are painted onto the face along their gaze, so they foreshorten with
// the skull and never appear on the back of the head.
const eyeDirection = (z: number): Vec => norm([0.78, -0.08, 0.66 * z]);
const eyeGaze = (z: number): Vec => norm([0.9, -0.02, 0.44 * z]);
const eyeSize = 20;
let eyeSurfaces: Vec[] | undefined;
function eyeSurface(z: number): Vec {
  eyeSurfaces ??= [-1, 1].map((side) => {
    const dir = eyeDirection(side);
    let lo = 0;
    let hi = 90;
    for (let i = 0; i < 40; i++) {
      const t = (lo + hi) / 2;
      if (headFur(mul(dir, t)) < 0) lo = t;
      else hi = t;
    }
    return mul(dir, lo);
  });
  return eyeSurfaces[z > 0 ? 1 : 0]!;
}

const earBase = (s: number): Vec => [0, 21, 26 * s];
const earTip = (s: number): Vec => [-4, 47, 38 * s];
const earDistance = ([x, y, z]: Vec, s: number) =>
  sdRoundCone([(x + 3) * 1.45, y, z], earBase(s), earTip(s), 15.5, 2.8) / 1.45;

// Tufts shape the silhouette only; shading uses the underlying smooth form.
let smoothNormals = false;

function headFur(h: Vec) {
  const [x, y, z] = h;
  let d = sdEllipsoid(x, y - 2, z, 38, 35, 47);
  d = smin(d, sdEllipsoid(x - 4, y + 13, Math.abs(z) - 21, 27, 22, 27), 12);
  d = smin(d, sdEllipsoid(x - 28, y + 15, z, 12, 9, 15), 7);
  // Ears: thin cones flattened front to back.
  for (const s of [-1, 1]) d = smin(d, earDistance(h, s), 5);
  // Cheek and crown tufts: object-space, so they stay with the fur.
  if (smoothNormals) return d;
  // Cheek tufts: object-space, so they stay with the fur at every angle.
  const tuft = (noise(x * 0.2 + 11, y * 0.3, z * 0.2) - 0.5) * 4.5;
  return d + tuft * clamp(Math.abs(z) / 44 - 0.55, 0, 1) * clamp(-y / 12 + 0.6, 0, 1);
}

function bodyFur(p: Vec) {
  const [x, y, z] = p;
  let d = torso(x, y, z);
  const local: Vec = [x, y, Math.abs(z)];
  d = smin(d, limbDistance(local, foreleg, 6), 10);
  d = smin(d, limbDistance(local, hindleg, 7), 9);
  return smin(d, limbDistance(p, tail, 5), 7);
}

function furDistance(p: Vec) {
  let d = bodyFur(p);
  const neck = sdRoundCone(p, neckBase, neckTopBody, 16, 15);
  d = smin(d, neck, 12);
  const h = headLocalFromBody(p);
  return smin(d, headFur(h), 12);
}

interface Hit {
  d: number;
  m: M;
}

const strapX = 5;
function strapDistance(p: Vec) {
  if (Math.abs(p[0] - strapX) > 6) return Infinity;
  return Math.max(bodyFur(p) - 1.8, Math.abs(p[0] - strapX) - 3.4);
}

function sceneDistance(p: Vec, out: Hit) {
  const fur = furDistance(p);
  out.d = fur;
  out.m = M.Fur;
  const h = headLocalFromBody(p);
  const nose = sdEllipsoid(h[0] - 39.6, h[1] + 15.5, h[2], 3.2, 2.8, 4.6);
  if (nose < out.d) {
    out.d = nose;
    out.m = M.Nose;
  }
  const [x, y, z] = p;
  const strap = strapDistance(p);
  if (strap < out.d) {
    out.d = strap;
    out.m = M.Strap;
  }
  const buckle = sdRoundBox(x - strapX, y + 4, z - 26.6, 4.2, 5, 1.6, 1.2);
  if (buckle < out.d) {
    out.d = buckle;
    out.m = M.Buckle;
  }
  const plate = sdRoundBox(x + 8, y - 26, z, 15, 6.5, 14, 3);
  if (plate < out.d) {
    out.d = plate;
    out.m = M.Plate;
  }
  for (const s of [-1, 1]) {
    const cy = y - thrusterY;
    const cz = z - thrusterZ * s;
    const radial = Math.hypot(cy, cz);
    // Barrel with a domed cap toward the head.
    const barrel = sdRoundCone(
      [x, cy, cz],
      [8, 0, 0],
      [nozzleExit + 9, 0, 0],
      thrusterRadius,
      thrusterRadius,
    );
    if (barrel < out.d) {
      out.d = barrel;
      out.m = M.Metal;
    }
    // Bell nozzle, open at the back.
    const u = clamp((nozzleExit + 9 - x) / 9);
    const bellRadius = mix(6, 7.6, u * u);
    const shell = Math.abs(radial - bellRadius + 0.6) - 1;
    const bell = Math.max(shell, Math.abs(x - (nozzleExit + 4.5)) - 4.5);
    if (bell < out.d) {
      out.d = bell;
      out.m = M.Nozzle;
    }
  }
  return out.d;
}

// --- exhaust ---------------------------------------------------------------

const flameLength = 96;
const flameRadius = 19;
const flameLayers = [
  { length: 0.3, width: 0.3, color: 0xfffbe8 },
  { length: 0.4, width: 0.42, color: 0xfff3b6 },
  { length: 0.5, width: 0.54, color: 0xffdb68 },
  { length: 0.58, width: 0.64, color: 0xffb938 },
  { length: 0.66, width: 0.72, color: 0xff8a24 },
  { length: 0.7, width: 0.76, color: 0xe65c29 },
  { length: 0.78, width: 0.84, color: 0x9660e0 },
  { length: 0.9, width: 0.93, color: 0x079de4 },
  { length: 1, width: 1, color: 0x28ceea },
] as const;

function teardrop(u: number) {
  if (u < 0) return 0;
  if (u < 0.17) return mix(0.36, 1, Math.sin((u / 0.17) * Math.PI * 0.5));
  return Math.pow(Math.max(0, (1 - u) / 0.83), 1.05);
}

// Plumes keep their on-screen length when they point toward the viewer.
function plumeLength() {
  const [f] = pose.body;
  return flameLength * Math.min(1.35, 1 / Math.max(0.01, Math.hypot(f[0], f[1])));
}

// Returns the hottest layer the ray passes through and its entry depth.
function exhaust(origin: Vec, dir: Vec): { layer: number; t: number } | undefined {
  const o = toBody(origin);
  const d = toBody(dir);
  const length = plumeLength();
  let best: number = flameLayers.length;
  let entry = Infinity;
  const xMin = nozzleExit - length - 6;
  const xMax = nozzleExit + 1;
  const reach = flameRadius * 1.35;
  const box = [
    [xMin, xMax],
    [thrusterY - reach, thrusterY + reach],
    [-thrusterZ - reach, thrusterZ + reach],
  ] as const;
  let t0 = -Infinity;
  let t1 = Infinity;
  for (let i = 0; i < 3; i++) {
    const [lo, hi] = box[i]!;
    if (Math.abs(d[i]!) < 1e-9) {
      if (o[i]! < lo || o[i]! > hi) return;
      continue;
    }
    let a = (lo - o[i]!) / d[i]!;
    let b = (hi - o[i]!) / d[i]!;
    if (a > b) [a, b] = [b, a];
    t0 = Math.max(t0, a);
    t1 = Math.min(t1, b);
  }
  if (t0 > t1) return;
  const ring = pose.phase;
  for (let t = t0; t <= t1; t += 0.5) {
    const x = o[0] + d[0] * t;
    const y = o[1] + d[1] * t;
    const z = o[2] + d[2] * t;
    const s = nozzleExit - x;
    if (s < 0) continue;
    for (const side of [-1, 1]) {
      const cy = y - thrusterY;
      const cz = z - thrusterZ * side;
      const u = s / length;
      // Flowing, loop-safe turbulence: an extra noise axis travels on a circle.
      const swirl = fbm(
        s * 0.1 + Math.cos(ring) * 1.6 + side * 13,
        cy * 0.14 + Math.sin(ring) * 1.6,
        cz * 0.14,
      );
      const angle = Math.atan2(cz, cy);
      const spikes = noise(
        s * 0.32 + Math.cos(ring) * 2.2 + side * 7,
        Math.cos(angle) * 2.6 + Math.sin(ring) * 2.2,
        Math.sin(angle) * 2.6 + 40,
      );
      const radial = Math.hypot(cy, cz) / flameRadius;
      const along = u * (1 + (swirl - 0.5) * 0.1);
      for (let k = 0; k < best; k++) {
        const layer = flameLayers[k]!;
        const lu = along / layer.length;
        if (lu > 1) continue;
        const outer = k / (flameLayers.length - 1);
        const edge =
          layer.width *
          teardrop(lu) *
          (1 + (swirl - 0.5) * 0.35 + (spikes - 0.5) * outer * outer * (0.25 + lu) * 0.9);
        if (radial <= edge) {
          best = k;
          if (t < entry) entry = t;
          break;
        }
      }
    }
  }
  return best < flameLayers.length ? { layer: best, t: entry } : undefined;
}

// --- shading ---------------------------------------------------------------

const coat = [
  0x1c0d29, 0x251133, 0x30133f, 0x3b184e, 0x47205e, 0x54296e, 0x5b3077, 0x60357e, 0x6a3c87,
  0x87509e,
];
const metal = [0x382d4b, 0x665571, 0x9986ac, 0xc6b6d8, 0xf0e9f8];
const band = [0x066d95, 0x0ea1c8, 0x28ceea, 0x6be4ff];
const strapColors = [0x190b24, 0x2e1f3f, 0x45355a, 0x5d4b74];
// A two-pixel outline just outside the cat and harness, darker than the coat's
// deepest shade. It crisps the silhouette on light backgrounds and blends into
// dark ones.
const outlineWidth = 2;
const outlineColor = 0x120a1c;
const light = norm([-0.5, 0.75, 0.45]);

function normalAt(p: Vec, f: (p: Vec) => number): Vec {
  const e = 0.35;
  return norm([
    f([p[0] + e, p[1], p[2]]) - f([p[0] - e, p[1], p[2]]),
    f([p[0], p[1] + e, p[2]]) - f([p[0], p[1] - e, p[2]]),
    f([p[0], p[1], p[2] + e]) - f([p[0], p[1], p[2] - e]),
  ]);
}

const scratch: Hit = { d: 0, m: M.None };
const sceneOnly = (p: Vec) => sceneDistance(p, scratch);

function occlusion(p: Vec, n: Vec) {
  let o = 0;
  let w = 1;
  for (const h of [2.5, 5, 9, 14]) {
    // The head does not shade the limbs, so paws under the chin stay readable.
    o += (h - bodyFur(add(p, mul(n, h)))) * w;
    w *= 0.6;
  }
  return clamp(1 - o * 0.045);
}

function lambert(nWorld: Vec) {
  return 0.5 * dot(nWorld, light) + 0.5;
}

function pick(ramp: readonly number[], v: number) {
  return ramp[clamp(Math.floor(v * ramp.length), 0, ramp.length - 1)]!;
}

function fromBody(v: Vec): Vec {
  const [f, u, r] = pose.body;
  return add(add(mul(f, v[0]), mul(u, v[1])), mul(r, v[2]));
}

// Each paw has toe creases on top and a main pad with three toe beans on the
// underside, which faces the belly side of the body.
function pawDecal(p: Vec): number | undefined {
  const local: Vec = [p[0], p[1], Math.abs(p[2])];
  for (const limb of [foreleg, hindleg]) {
    const n = limb.points.length;
    const tip = limb.points[n - 1]!;
    const axis = norm(sub(tip, limb.points[n - 2]!));
    const q = sub(local, tip);
    const along = dot(q, axis);
    if (along < -7 || along > 8 || Math.hypot(...q) > 11) continue;
    const radial = sub(q, mul(axis, along));
    const down = norm(sub([0, -1, 0], mul(axis, -axis[1])));
    const facing = dot(radial, down) / Math.max(0.01, Math.hypot(...radial));
    const across = dot(radial, cross(axis, down));
    if (facing > 0.45) {
      for (const c of [-2.7, 0, 2.7]) {
        if (Math.hypot(across - c, along - (c === 0 ? 4 : 3)) < 1.4) return 0xdfb4df;
      }
      const pad = Math.hypot(across / 3.4, (along + 2.2) / 2.4);
      if (pad < 1) return pad > 0.75 ? 0x87509e : 0xad72c6;
      return;
    }
    if (facing < 0.15 && along > -5.5 && Math.abs(Math.abs(across) - 2.3) < 0.55) return coat[0];
    return;
  }
}

const debugParts = process.env.PORTAL_CAT_PARTS === "1";
function partColor(p: Vec) {
  const local: Vec = [p[0], p[1], Math.abs(p[2])];
  const parts: [number, number][] = [
    [torso(p[0], p[1], p[2]), 0x6a3c87],
    [limbDistance(local, foreleg, 6), p[2] > 0 ? 0x2a9d4a : 0x1d6a33],
    [limbDistance(local, hindleg, 7), p[2] > 0 ? 0xd08a2a : 0x8a5a1a],
    [limbDistance(p, tail, 5), 0xc03030],
    [sdRoundCone(p, neckBase, neckTopBody, 16, 15), 0x3060c0],
    [headFur(headLocalFromBody(p)), 0x9090a0],
  ];
  return parts.reduce((a, b) => (b[0] < a[0] ? b : a))[1];
}

function furColor(p: Vec, nBody: Vec) {
  const n = fromBody(nBody);
  const h = headLocalFromBody(p);
  if (debugParts) {
    const c = partColor(p);
    const l = lambert(n);
    const f = 0.45 + 0.55 * l;
    return (
      (Math.round(((c >> 16) & 255) * f) << 16) |
      (Math.round(((c >> 8) & 255) * f) << 8) |
      Math.round((c & 255) * f)
    );
  }
  // Inner ears: lavender triangles on the front of each ear.
  for (const s of [-1, 1]) {
    const base = earBase(s);
    const tip = earTip(s);
    const along = clamp((h[1] - base[1]) / (tip[1] - base[1]));
    const center = mix(base[2], tip[2], along);
    const width = mix(10, 0.8, along);
    const onEar = earDistance(h, s) < 1 && h[0] > 1;
    if (onEar && h[1] > base[1] + 5 && Math.abs(h[2] - center) < width && along < 0.86) {
      return lambert(n) > 0.55 ? 0xdfb4df : 0xad72c6;
    }
  }
  for (const s of [1, -1]) {
    const eye = eyeColor(h, s);
    if (eye !== undefined) return eye;
  }
  // Mouth: a small dark "w" under the nose on the muzzle.
  if (h[0] > 26) {
    const mz = Math.abs(h[2]);
    const arc = Math.abs(Math.hypot(mz - 4, h[1] + 19.5) - 4);
    const stem = mz < 0.8 && h[1] < -17.5 && h[1] > -20;
    if ((arc < 0.85 && h[1] < -19.2) || stem) return 0x1c0d29;
  }
  const paw = pawDecal(p);
  if (paw !== undefined) return paw;
  // Fur clusters run along the body; head fur uses head coordinates.
  const isHead = headFur(h) < 1.5;
  const ao = isHead ? 1 : occlusion(p, nBody);
  const streak = isHead
    ? fbm(h[0] * 0.05 + 30, h[1] * 0.08, h[2] * 0.08)
    : fbm(p[0] * 0.03, p[1] * 0.1 + 5, p[2] * 0.1);
  let v = Math.pow(lambert(n), 1.35) * 0.88 - 0.13;
  v = v * mix(0.3, 1, ao) + (streak - 0.5) * 0.12;
  // Soft fill keeps the face readable around the muzzle and mouth.
  if (isHead) v = v * 0.75 + 0.15 + 0.08 * clamp(dot(n, [0, 0, 1]));
  return pick(coat, clamp(v));
}

function eyeColor(h: Vec, s: number): number | undefined {
  const g = eyeGaze(s);
  const c = eyeSurface(s);
  const q = sub(h, c);
  if (dot(q, g) < -14) return;
  const up = norm(sub([0, 1, 0], mul(g, g[1])));
  const across = cross(g, up);
  // Positive ux points toward the nose on both eyes.
  const ux = -dot(q, across) * s;
  const uy = dot(q, up);
  const r = Math.hypot(ux, uy) / eyeSize;
  if (r > 1) return;
  if (r > 0.9) return 0x080813;
  const iris = Math.hypot(ux - 1.4, uy - 1.6) / (eyeSize * 0.86);
  if (iris > 1) return uy < -4 ? 0xfff4dc : 0xfffbe8;
  if (Math.hypot(ux + 5.4, uy - 7.4) < 3.6) return 0xffffff;
  if (Math.hypot(ux - 4.6, uy + 0.6) < 1.8) return 0xffffff;
  const pupil = Math.hypot(ux - 2.6, uy - 2.4) / (eyeSize * 0.46);
  if (pupil < 1) return iris > 0.6 && pupil > 0.86 ? 0x080813 : 0x04142e;
  const cyan = [0x1250a6, 0x156fd3, 0x079de4, 0x1dcaf3, 0x6be4ff];
  const violet = [0x5630a4, 0x8250d4, 0x9660e0, 0xad79ed, 0xd6adff];
  const ramp = s > 0 ? cyan : violet;
  return pick(ramp, clamp(0.45 - uy / 34 + (iris > 0.9 ? -0.35 : 0)));
}

// --- rendering ---------------------------------------------------------------

export interface Layers {
  width: number;
  height: number;
  cat: Uint32Array;
  equipment: Uint32Array;
  exhaust: Uint32Array;
}

export function renderFrame(
  next: Pose,
  width: number,
  height: number,
  cx: number,
  cy: number,
): Layers {
  setPose(next);
  const size = width * height;
  const color = new Uint32Array(size);
  const material = new Uint8Array(size);
  const depth = new Float32Array(size).fill(-Infinity);
  const flame = new Uint32Array(size);
  const dir: Vec = [0, 0, -1];
  const hit: Hit = { d: 0, m: M.None };
  for (let j = 0; j < height; j++) {
    for (let i = 0; i < width; i++) {
      const wx = (i + 0.5 - cx) / next.scale;
      const wy = (cy - j - 0.5) / next.scale;
      if (wx * wx + wy * wy > 190 * 190) continue;
      const origin: Vec = [wx, wy, 220];
      const k = j * width + i;
      let t = 0;
      const o = toBody(origin);
      let p: Vec = o;
      const d = toBody(dir);
      let found = false;
      for (let step = 0; step < 160 && t < 440; step++) {
        p = [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t];
        const dist = sceneDistance(p, hit);
        if (dist < 0.05) {
          found = true;
          break;
        }
        t += Math.max(dist * 0.9, 0.05);
      }
      const plume = exhaust(origin, dir);
      if (found) {
        depth[k] = 220 - t;
        material[k] = hit.m;
        smoothNormals = true;
        const n = normalAt(p, sceneOnly);
        smoothNormals = false;
        color[k] = shade(hit.m, p, n);
      }
      if (plume && (!found || plume.t < t)) {
        flame[k] = flameLayers[plume.layer]!.color;
      }
    }
  }
  const layers = postProcess(width, height, color, material, depth, flame);
  drawWhiskers(layers, depth, next, cx, cy);
  return layers;
}

const whiskers = [-1, 1].flatMap((side) =>
  [0, 1, 2].map((k) => ({
    from: [32, -21 + k * 2.6, 13 * side] as Vec,
    to: [28 - k * 5, -17 - k * 7, 58 * side] as Vec,
  })),
);

function drawWhiskers(layers: Layers, depth: Float32Array, next: Pose, cx: number, cy: number) {
  const [hf, hu, hr] = next.head;
  const world = (h: Vec) =>
    add(next.headCenter, add(add(mul(hf, h[0]), mul(hu, h[1])), mul(hr, h[2])));
  for (const { from, to } of whiskers) {
    const a = world(from);
    const b = world(to);
    const steps = Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1]) * next.scale * 2) + 1;
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const p = add(a, mul(sub(b, a), t));
      const x = Math.floor(cx + p[0] * next.scale);
      const y = Math.floor(cy - p[1] * next.scale);
      if (x < 0 || y < 0 || x >= layers.width || y >= layers.height) continue;
      const k = y * layers.width + x;
      if (p[2] < depth[k]! - 0.5 || layers.exhaust[k]) continue;
      layers.equipment[k] = 0;
      layers.cat[k] = (t > 0.55 ? 0xb481bd : 0xce96e1) | 0xff000000;
    }
  }
}

function shade(m: M, p: Vec, n: Vec): number {
  const nWorld = fromBody(n);
  const l = lambert(nWorld);
  switch (m) {
    case M.Fur:
      return furColor(p, n);
    case M.Nose:
      return l > 0.62 ? 0xdfb4df : 0xad72c6;
    case M.Strap:
      return pick(strapColors, l * 0.95);
    case M.Buckle:
      return pick(metal, l);
    case M.Plate:
      return pick(metal, l * 0.6);
    case M.Nozzle: {
      const radial = Math.hypot(p[1] - thrusterY, p[2] - thrusterZ * Math.sign(p[2]));
      return radial < 6 ? 0x382d4b : pick(metal, l * 0.75);
    }
    case M.Metal: {
      const x = p[0];
      const spec = Math.pow(clamp(dot(nWorld, norm(add(light, [0, 0, 1])))), 18);
      if ((x > 0 && x < 4) || (x > -21 && x < -17)) return pick(band, l * 0.9 + spec * 0.3);
      return pick(metal, clamp(l * 0.85 + spec * 0.5));
    }
    default:
      return 0;
  }
}

function postProcess(
  width: number,
  height: number,
  color: Uint32Array,
  material: Uint8Array,
  depth: Float32Array,
  flame: Uint32Array,
): Layers {
  const cat = new Uint32Array(width * height);
  const equipment = new Uint32Array(width * height);
  const exhaustLayer = new Uint32Array(width * height);
  const isCat = (m: number) => m === M.Fur || m === M.Nose;
  const at = (i: number, j: number) =>
    i < 0 || j < 0 || i >= width || j >= height ? -1 : j * width + i;
  for (let j = 0; j < height; j++) {
    for (let i = 0; i < width; i++) {
      const k = j * width + i;
      if (flame[k]) {
        exhaustLayer[k] = flame[k]! | 0xff000000;
        continue;
      }
      const m = material[k]!;
      if (!m) continue;
      let c = color[k]!;
      let outer = false;
      let behind = false;
      for (const [di, dj] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ] as const) {
        const n = at(i + di, j + dj);
        if (n < 0 || (!material[n] && !flame[n])) outer = true;
        else if (material[n] && depth[n]! - depth[k]! > 3.5) behind = true;
      }
      if (m === M.Fur) {
        if (behind) c = coat[0]!;
      } else if (!isCat(m) && (outer || behind)) {
        c = m === M.Strap ? strapColors[0]! : metal[0]!;
      }
      (isCat(m) ? cat : equipment)[k] = c | 0xff000000;
    }
  }
  // The self-lit exhaust is left unoutlined.
  let ring = new Uint8Array(width * height);
  for (let k = 0; k < ring.length; k++) ring[k] = material[k] && !flame[k] ? 1 : 0;
  for (let pass = 0; pass < outlineWidth; pass++) {
    const next = ring.slice();
    for (let j = 0; j < height; j++) {
      for (let i = 0; i < width; i++) {
        const k = j * width + i;
        if (ring[k] || material[k] || flame[k]) continue;
        const touches = [at(i + 1, j), at(i - 1, j), at(i, j + 1), at(i, j - 1)].some(
          (n) => n >= 0 && ring[n],
        );
        if (!touches) continue;
        next[k] = 1;
        cat[k] = outlineColor | 0xff000000;
      }
    }
    ring = next;
  }
  return { width, height, cat, equipment, exhaust: exhaustLayer };
}

// --- poses -----------------------------------------------------------------

// The cat flies out of the portal toward the viewer, banks through a
// three-quarter profile, then shows its back.
// Close to head-on would hide the pack and plumes behind the big head, so the
// cat starts angled just enough to show both barrels, with its face to us.
const frontBody = basis([0.78, 0.18, 0.6], [-0.1, 0.82, 0.56]);
const frontHead = basis([0.4, 0, 1], [0, 1, -0.02]);
const profileBody = basis([0.86, 0.46, 0.18], [-0.4, 0.7, 0.58]);
const profileHead = basis([0.42, -0.02, 0.92], [0.04, 1, 0.02]);
const rearBody = basis([0.04, 0.78, -0.62], [0, 0.62, 0.78]);
const rearHead = basis([0.02, 0.3, -0.95], [0, 0.95, 0.3]);

function headOffset(body: Basis, head: Basis): Vec {
  // Neck top is attached to the body neck; keep the head centred on it.
  const [hf, hu, hr] = head;
  const top = add(add(mul(hf, neckTop[0]), mul(hu, neckTop[1])), mul(hr, neckTop[2]));
  const [bf, bu, br] = body;
  // The neck stretches forward in flight, so the head leads the shoulders.
  const base = add(add(mul(bf, neckBase[0] + 10), mul(bu, neckBase[1] - 2)), mul(br, neckBase[2]));
  return sub(base, top);
}

const profileAt = 0.22;

export function turnPose(a: number, phase: number, scale: number): Pose {
  const t = smooth(clamp(a));
  const body =
    t < profileAt
      ? turnBasis(frontBody, t / profileAt, profileBody)
      : turnBasis(profileBody, (t - profileAt) / (1 - profileAt), rearBody);
  // The head keeps the viewer in sight until the profile, then leads the body
  // and ends looking ahead into the portal.
  const h = smooth(clamp(a * 1.1));
  const head =
    h < profileAt
      ? turnBasis(frontHead, h / profileAt, profileHead)
      : turnBasis(profileHead, (h - profileAt) / (1 - profileAt), rearHead);
  return { body, head, headCenter: headOffset(body, head), scale, phase };
}

// --- PNG -------------------------------------------------------------------

export function encodePng(width: number, height: number, pixels: Uint32Array) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let j = 0; j < height; j++) {
    raw[j * (width * 4 + 1)] = 0;
    for (let i = 0; i < width; i++) {
      const v = pixels[j * width + i]!;
      const o = j * (width * 4 + 1) + 1 + i * 4;
      raw[o] = (v >> 16) & 255;
      raw[o + 1] = (v >> 8) & 255;
      raw[o + 2] = v & 255;
      raw[o + 3] = (v >>> 24) & 255;
    }
  }
  const chunk = (type: string, data: Buffer) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(data.length, 0);
    head.write(type, 4, "ascii");
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(Bun.hash.crc32(Buffer.concat([head.subarray(4), data])) >>> 0, 0);
    return Buffer.concat([head, data, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

export function composite({ width, height, cat, equipment, exhaust: plume }: Layers) {
  const out = new Uint32Array(width * height);
  for (let k = 0; k < out.length; k++) out[k] = plume[k] || cat[k] || equipment[k] || 0;
  return out;
}

function debugViews(out: string) {
  const views: [string, Basis][] = [
    ["side", basis([1, 0, 0], [0, 1, 0])],
    ["top", basis([1, 0, 0], [0, 0, 1])],
    ["front", basis([0, 0, 1], [0, 1, 0])],
    ["back", basis([0, 0, -1], [0, 1, 0])],
  ];
  return views.map(async ([name, body], i) => {
    const head = body;
    const layers = renderFrame(
      { body, head, headCenter: headOffset(body, head), scale: 1, phase: 0 },
      320,
      256,
      160,
      128,
    );
    await Bun.write(`${out}/debug-${i}-${name}.png`, encodePng(320, 256, composite(layers)));
  });
}

// Master geometry. The anchor (body origin) sits at the same place in the
// 256 × 256 thrust tile and the 320 × 256 turn tile, so the turn's first cel
// lines up with the thrust loop.
const scale = 1.04;
const tile = 256;
const turnWidth = 320;
const anchor = { x: 154, y: 144 };
const flickerFrames = 6;
const flickerMs = 75;
// Bank cel durations (ms); keep in sync with the cues in web/src/portal/flight.ts.
const bankMs = Array.from({ length: 18 }, () => 48);

const phaseOf = (frame: number) => (2 * Math.PI * (frame % flickerFrames)) / flickerFrames;

async function writeLayers(dir: string, name: string, layers: Layers) {
  for (const key of ["cat", "equipment", "exhaust"] as const) {
    await Bun.write(
      `${dir}/${name}-${key}.png`,
      encodePng(layers.width, layers.height, layers[key]),
    );
  }
}

async function buildMasters(root: string, aseprite: string) {
  const { mkdtemp, rm } = await import("node:fs/promises");
  const { tmpdir } = await import("node:os");
  const work = await mkdtemp(`${tmpdir()}/portal-cat-`);
  try {
    for (let f = 0; f < flickerFrames; f++) {
      const layers = renderFrame(turnPose(0, phaseOf(f), scale), tile, tile, anchor.x, anchor.y);
      await writeLayers(work, `thrust-${f}`, layers);
    }
    // The app header keeps the three-quarter profile, which reads at icon size.
    const profile = { body: profileBody, head: profileHead, scale, phase: 0 };
    await writeLayers(
      work,
      "app-0",
      renderFrame(
        { ...profile, headCenter: headOffset(profileBody, profileHead) },
        tile,
        tile,
        anchor.x,
        anchor.y,
      ),
    );
    const bank = bankMs.length;
    for (let f = 0; f < bank + flickerFrames; f++) {
      const a = Math.min(1, f / (bank - 1));
      const offset = anchor.x + (turnWidth - tile) / 2;
      const layers = renderFrame(turnPose(a, phaseOf(f), scale), turnWidth, tile, offset, anchor.y);
      await writeLayers(work, `turn-${f}`, layers);
    }
    const art = `${root}/docs/design/explorations/art`;
    const jobs = [
      {
        out: `${art}/portal-cat/portal-cat.aseprite`,
        width: tile,
        prefix: "thrust",
        frames: Array.from({ length: flickerFrames }, () => flickerMs),
        tags: [["Thrust", 1, flickerFrames]],
      },
      {
        out: `${art}/portal-cat/portal-cat-turn.aseprite`,
        width: turnWidth,
        prefix: "turn",
        frames: [...bankMs, ...Array.from({ length: flickerFrames }, () => flickerMs)],
        tags: [
          ["Turn", 1, bank + flickerFrames],
          ["Bank away", 1, bank],
          ["Rear thrust", bank + 1, bank + flickerFrames],
        ],
      },
      {
        out: `${art}/app-cat/app-cat.aseprite`,
        width: tile,
        prefix: "app",
        frames: [flickerMs],
        tags: [],
      },
    ];
    for (const job of jobs) {
      const lua = `${work}/build.lua`;
      await Bun.write(lua, masterScript(work, job));
      const child = Bun.spawn([aseprite, "--batch", "--script", lua], {
        stdout: "inherit",
        stderr: "inherit",
      });
      if (await child.exited) throw new Error(`Aseprite could not build ${job.out}`);
    }
  } finally {
    await rm(work, { recursive: true, force: true });
  }
}

function masterScript(
  work: string,
  job: {
    out: string;
    width: number;
    prefix: string;
    frames: number[];
    tags: (string | number)[][];
  },
) {
  const durations = job.frames.map((ms) => (ms / 1000).toString()).join(", ");
  const tags = job.tags.map(([name, from, to]) => `{ "${name}", ${from}, ${to} }`).join(", ");
  return `
local sprite = Sprite(${job.width}, ${tile}, ColorMode.RGB)
local durations = { ${durations} }
for i = 2, #durations do sprite:newEmptyFrame() end
for i, d in ipairs(durations) do sprite.frames[i].duration = d end
local names = { "Harness and thrusters", "Cat", "Exhaust" }
local keys = { "equipment", "cat", "exhaust" }
local base = sprite.layers[1]
base.name = names[1]
local layers = { base, sprite:newLayer(), sprite:newLayer() }
layers[2].name = names[2]
layers[3].name = names[3]
for f = 1, #durations do
  for l = 1, 3 do
    local image = Image{ fromFile = "${work}/${job.prefix}-" .. (f - 1) .. "-" .. keys[l] .. ".png" }
    sprite:newCel(layers[l], f, image, Point(0, 0))
  end
end
for _, t in ipairs({ ${tags} }) do
  local tag = sprite:newTag(t[2], t[3])
  tag.name = t[1]
end
sprite:saveAs("${job.out}")
`;
}

async function asepritePath(root: string) {
  const { parseEnv } = await import("node:util");
  const local = Bun.file(`${root}/.env.aseprite`);
  const path =
    Bun.env.ASEPRITE_PATH ||
    ((await local.exists()) ? parseEnv(await local.text()).ASEPRITE_PATH : undefined);
  if (!path) throw new Error("Set ASEPRITE_PATH in .env.aseprite or your environment.");
  return path;
}

if (import.meta.main) {
  const { mkdir } = await import("node:fs/promises");
  const [mode = "--masters", out = "/tmp/portal-cat-preview", stops = "0,0.25,0.5,0.75,1"] =
    process.argv.slice(2);
  if (mode === "--masters") {
    const root = `${import.meta.dir}/..`;
    await buildMasters(root, await asepritePath(root));
  } else if (mode === "--debug") {
    await mkdir(out, { recursive: true });
    await Promise.all(debugViews(out));
  } else if (mode === "--preview") {
    await mkdir(out, { recursive: true });
    for (const [i, a] of stops.split(",").map(Number).entries()) {
      const layers = renderFrame(
        turnPose(a, phaseOf(i), scale),
        turnWidth,
        tile,
        anchor.x + 32,
        anchor.y,
      );
      await Bun.write(`${out}/frame-${i}.png`, encodePng(turnWidth, tile, composite(layers)));
    }
  } else {
    throw new Error(
      "Usage: render-portal-cat.ts [--masters | --preview <dir> <turns> | --debug <dir>]",
    );
  }
}
