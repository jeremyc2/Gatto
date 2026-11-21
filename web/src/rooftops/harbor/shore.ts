import { clamp, type ArtPath, type Point2, type Point3, type Rect4 } from "../types";
import { blendInk, pushShape, View, WATER } from "./projection";

export const STREET_DEPTH = 270;
export const COURT_REVEAL_DEPTH = -18;
const WALL_TOP = -7;
const DOCK_TOP = -5;

export function streetGrade(view: View): number {
  return view.waterline() - 24;
}

function ease(x: number): number {
  const t = clamp(x, 0, 1);
  return t * t * (3 - 2 * t);
}

type PlantKind = "broadleaf" | "evergreen" | "shrub" | "bush" | "flowers";

export function recreation(
  view: View,
  cameraDepth: number,
  frontage: [Point2, Point2] | null,
): [ArtPath[], ArtPath[]] {
  const clarity = ease((cameraDepth - COURT_REVEAL_DEPTH) / 185);
  if (clarity === 0) return [[], []];
  const park = new Park(view, frontage, clarity, ease((cameraDepth - 98) / 270));
  park.lawn();
  park.patch([-130, 661, 265, 162], 0x343745);
  park.pitch([-120, 672, 101, 139]);
  park.court([16, 677, 109, 129]);
  park.patch([-19, 661, 35, 162], 0x3a3c48);
  park.patch([-19, 806, 35, 1.2], 0x54515b);
  return [park.surface, park.fixtures];
}

class Park {
  readonly surface: ArtPath[] = [];
  readonly fixtures: ArtPath[] = [];
  private readonly view: View;
  private readonly frontage: [Point2, Point2] | null;
  private readonly clarity: number;
  private readonly detail: number;

  constructor(view: View, frontage: [Point2, Point2] | null, clarity: number, detail: number) {
    this.view = view;
    this.frontage = frontage;
    this.clarity = clarity;
    this.detail = detail;
  }

  private point([x, y, z]: Point3): Point3 {
    return [this.view.xAtDepth(this.view.width * 0.5, z) + x, streetGrade(this.view) - y, z];
  }

  private ground(points: readonly Point2[], ink: number): void {
    this.view.face(
      this.surface,
      points.map(([x, z]) => this.point([x, 0, z])),
      ink,
      this.clarity,
    );
  }

  patch([x, z, w, d]: Rect4, ink: number): void {
    this.ground(
      [
        [x, z],
        [x + w, z],
        [x + w, z + d],
        [x, z + d],
      ],
      ink,
    );
  }

  private outline([x, z, w, d]: Rect4, ink: number): void {
    const PAINT = 0.85;
    this.patch([x, z, w, PAINT], ink);
    this.patch([x, z + d - PAINT, w, PAINT], ink);
    this.patch([x, z + PAINT, PAINT, d - 2 * PAINT], ink);
    this.patch([x + w - PAINT, z + PAINT, PAINT, d - 2 * PAINT], ink);
  }

  private arc([x, z]: Point2, radius: number, [from, to]: Point2, ink: number): void {
    const pts: Point2[] = [];
    for (let i = 0; i <= 18; i++) {
      const theta = from + ((to - from) * i) / 18;
      pts.push([x + (radius + 0.42) * Math.cos(theta), z + (radius + 0.42) * Math.sin(theta)]);
    }
    for (let i = 18; i >= 0; i--) {
      const theta = from + ((to - from) * i) / 18;
      pts.push([x + (radius - 0.42) * Math.cos(theta), z + (radius - 0.42) * Math.sin(theta)]);
    }
    this.ground(pts, ink);
  }

  private fixture(points: readonly Point3[], ink: number): void {
    this.view.face(
      this.fixtures,
      points.map((pt) => this.point(pt)),
      ink,
      this.clarity,
    );
  }

  private rail(a: Point3, b: Point3, width: number, ink: number): void {
    this.view.line(this.fixtures, this.point(a), this.point(b), width, ink, this.clarity);
  }

  private place(feet: readonly Point2[], draw: () => void): void {
    let visibility = 1;
    if (this.view.camera && this.frontage) {
      const [[ax, ay], [bx, by]] = this.frontage;
      for (const [x, z] of feet) {
        const pt = this.view.camera.point(this.point([x, 0, z]));
        if (!pt) {
          visibility = 0;
          break;
        }
        const edgeY = ay + ((by - ay) * (pt[0] - ax)) / (bx - ax);
        visibility = Math.min(visibility, ease((pt[1] - edgeY - 1.5) / 7));
      }
    }
    if (visibility > 0) {
      const sStart = this.surface.length;
      const fStart = this.fixtures.length;
      draw();
      for (let i = sStart; i < this.surface.length; i++) {
        this.surface[i]!.opacity *= visibility;
      }
      for (let i = fStart; i < this.fixtures.length; i++) {
        this.fixtures[i]!.opacity *= visibility;
      }
    }
  }

  private accent(surface: number, ink: number): number {
    return blendInk(surface, ink, 0.35 + 0.65 * this.detail);
  }

  lawn(): void {
    const GRASS = 0x273e36;
    this.ground(
      [
        [-345, 321],
        [-292, 298],
        [295, 298],
        [363, 326],
        [382, 1085],
        [332, 1145],
        [-326, 1145],
        [-376, 1083],
      ],
      GRASS,
    );
    const patches: readonly Rect4[] = [
      [-319, 328, 120, 50],
      [117, 315, 137, 58],
      [-252, 409, 80, 62],
      [-126, 383, 61, 53],
      [85, 417, 91, 55],
      [213, 442, 68, 82],
      [-294, 534, 86, 45],
      [-176, 574, 71, 57],
      [34, 520, 105, 43],
      [173, 580, 108, 67],
      [-282, 686, 78, 62],
      [170, 732, 119, 52],
      [-195, 778, 44, 54],
      [45, 816, 119, 24],
      [-308, 868, 110, 67],
      [-141, 850, 89, 110],
      [45, 882, 100, 66],
      [226, 886, 120, 82],
      [-286, 1007, 96, 70],
      [-112, 1045, 112, 68],
      [94, 1010, 142, 94],
    ];
    const patchInk = this.accent(GRASS, 0x2e453a);
    for (const [x, z, w, d] of patches) {
      this.ground(
        [
          [x, z + d * 0.3],
          [x + w * 0.42, z],
          [x + w, z + d * 0.2],
          [x + w * 0.86, z + d],
          [x + w * 0.15, z + d],
        ],
        patchInk,
      );
    }
    const blades = this.accent(GRASS, 0x3a5041);
    for (const [x, z, w, d] of [
      [-227, 340, 131, 250],
      [45, 335, 193, 288],
      [-253, 842, 440, 250],
    ] as const) {
      for (let i = 0; i < 24; i++) {
        const u = ((i * 37) % 101) / 101;
        const v = ((i * 19) % 43) / 43;
        const px = x + w * u;
        const pz = z + d * v;
        const span = 2 + (i % 4);
        this.ground(
          [
            [px, pz],
            [px + span, pz + 0.8],
            [px + 0.4, pz + 1.5],
          ],
          blades,
        );
      }
    }
    this.patch([-10, 298, 21, 366], 0x363944);

    const plants: readonly [PlantKind, Point3][] = [
      ["broadleaf", [77, 1076, 34]],
      ["evergreen", [-107, 1063, 32]],
      ["bush", [122, 1051, 9]],
      ["flowers", [-44, 1035, 5]],
      ["evergreen", [230, 1004, 31]],
      ["broadleaf", [-247, 997, 34]],
      ["shrub", [-194, 985, 11]],
      ["flowers", [144, 958, 5]],
      ["broadleaf", [-79, 941, 26]],
      ["bush", [48, 925, 10]],
      ["evergreen", [194, 913, 28]],
      ["flowers", [-133, 889, 4]],
      ["bush", [152, 869, 8]],
      ["shrub", [-157, 834, 8]],
      ["broadleaf", [-199, 791, 27]],
      ["evergreen", [203, 779, 25]],
      ["bush", [-150, 708, 9]],
      ["flowers", [149, 709, 4]],
      ["broadleaf", [166, 639, 24]],
      ["bush", [39, 634, 7]],
      ["shrub", [91, 624, 8]],
      ["flowers", [-82, 625, 4]],
      ["broadleaf", [-27, 597, 15]],
      ["broadleaf", [-219, 583, 28]],
      ["evergreen", [-69, 547, 21]],
      ["broadleaf", [68, 533, 18]],
      ["bush", [-147, 530, 9]],
      ["flowers", [-83, 511, 3.6]],
      ["broadleaf", [118, 488, 22]],
      ["flowers", [155, 472, 4]],
      ["flowers", [43, 466, 3.8]],
      ["bush", [-33, 451, 5.5]],
      ["evergreen", [-177, 442, 23]],
      ["shrub", [57, 415, 6.5]],
      ["shrub", [84, 397, 6]],
      ["flowers", [-52, 375, 4]],
      ["flowers", [31, 361, 3.6]],
      ["shrub", [-113, 350, 7]],
    ];
    for (const [kind, [x, z, height]] of plants) {
      this.place([[x, z]], () => this.plant(kind, [x, z], height));
    }
  }

  private plant(kind: PlantKind, [x, z]: Point2, size: number): void {
    const width =
      kind === "broadleaf"
        ? size * 0.53
        : kind === "evergreen"
          ? size * 0.37
          : kind === "shrub"
            ? size * 1.05
            : kind === "bush"
              ? size * 1.65
              : size * 1.55;
    this.ground(
      [
        [x - width * 0.7, z],
        [x + width * 0.65, z - width * 0.3],
        [x + width * 1.25, z + width * 0.55],
        [x - width * 0.3, z + width * 0.6],
      ],
      0x20332f,
    );
    const foliage = (silhouette: readonly Point2[], ink: number) => {
      this.fixture(
        silhouette.map(([u, v]) => [x + width * u, size * v, z]),
        ink,
      );
    };
    switch (kind) {
      case "broadleaf":
        foliage(
          [
            [-0.09, 0],
            [0.07, 0],
            [0.04, 0.55],
            [-0.06, 0.55],
          ],
          0x484740,
        );
        foliage(
          [
            [-0.96, 0.49],
            [-0.86, 0.69],
            [-0.59, 0.76],
            [-0.37, 0.94],
            [0.03, 1],
            [0.43, 0.88],
            [0.65, 0.84],
            [0.92, 0.71],
            [1, 0.49],
            [0.74, 0.35],
            [0.18, 0.31],
            [-0.49, 0.35],
          ],
          0x2c4839,
        );
        foliage(
          [
            [-0.88, 0.55],
            [-0.64, 0.71],
            [-0.39, 0.73],
            [-0.17, 0.91],
            [0.25, 0.92],
            [0.53, 0.75],
            [0.35, 0.55],
            [-0.3, 0.44],
          ],
          0x365443,
        );
        break;
      case "evergreen":
        foliage(
          [
            [-0.08, 0],
            [0.08, 0],
            [0.04, 0.42],
            [-0.04, 0.42],
          ],
          0x454442,
        );
        foliage(
          [
            [0, 1],
            [0.55, 0.68],
            [0.37, 0.68],
            [0.77, 0.48],
            [0.57, 0.49],
            [1, 0.24],
            [0.5, 0.19],
            [-0.42, 0.2],
            [-1, 0.26],
            [-0.54, 0.49],
            [-0.77, 0.49],
            [-0.39, 0.68],
            [-0.57, 0.68],
          ],
          0x233c35,
        );
        foliage(
          [
            [-0.82, 0.27],
            [-0.42, 0.5],
            [-0.63, 0.49],
            [0, 0.94],
            [0.16, 0.38],
          ],
          0x2e4a40,
        );
        break;
      case "shrub":
        foliage(
          [
            [-1, 0.15],
            [-0.84, 0.58],
            [-0.48, 0.67],
            [-0.3, 0.95],
            [0.23, 1],
            [0.48, 0.64],
            [0.68, 0.73],
            [0.95, 0.45],
            [1, 0.16],
            [0.46, 0],
            [-0.55, 0],
          ],
          0x3b5140,
        );
        foliage(
          [
            [-0.87, 0.42],
            [-0.48, 0.58],
            [-0.28, 0.87],
            [0.06, 0.91],
            [0.33, 0.6],
            [-0.16, 0.35],
          ],
          0x475b46,
        );
        break;
      case "bush":
        for (const [u, height, ink] of [
          [-0.45, 0.65, 0x29433b],
          [0.46, 0.79, 0x29463b],
          [0.03, 1, 0x324f41],
        ] as const) {
          foliage(
            [
              [u - 0.52, 0],
              [u - 0.49, height * 0.55],
              [u - 0.22, height * 0.93],
              [u + 0.15, height],
              [u + 0.47, height * 0.68],
              [u + 0.51, 0],
            ],
            ink,
          );
        }
        break;
      case "flowers":
        foliage(
          [
            [-0.94, 0],
            [-0.58, 0.55],
            [-0.13, 0.39],
            [0.4, 0.62],
            [0.94, 0.15],
            [0.38, 0],
          ],
          0x425a44,
        );
        for (const [u, v] of [
          [-0.64, 0.55],
          [-0.2, 0.76],
          [0.16, 0.42],
          [0.54, 0.64],
        ] as const) {
          foliage(
            [
              [u - 0.13, v],
              [u - 0.07, v + 0.2],
              [u + 0.08, v + 0.24],
              [u + 0.14, v + 0.04],
              [u + 0.04, v - 0.1],
            ],
            u < 0 ? 0x7f6574 : 0x9c7c82,
          );
        }
        break;
    }
  }

  pitch([x, z, w, d]: Rect4): void {
    const center = x + w * 0.5;
    const middle = z + d * 0.5;
    const grass = 0x34483c;
    const paint = this.accent(grass, 0x849386);
    this.patch([x - 5, z - 5, w + 10, d + 10], 0x293c33);
    this.patch([x, z, w, d], grass);
    const stripeInk = this.accent(grass, 0x3b5144);
    for (const stripe of [0, 2, 4]) {
      this.patch([x, z + (stripe * d) / 6, w, d / 6], stripeInk);
    }
    this.outline([x, z, w, d], paint);
    this.patch([x, middle - 0.42, w, 0.85], paint);
    this.arc([center, middle], 13, [0, Math.PI], paint);
    this.arc([center, middle], 13, [Math.PI, 2 * Math.PI], paint);
    this.patch([center - 0.8, middle - 0.8, 1.6, 1.6], paint);
    for (const [end, sign] of [
      [z + d, -1],
      [z, 1],
    ] as const) {
      const boxZ = Math.min(end, end + sign * 21);
      this.outline([center - 25, boxZ, 50, 21], paint);
      const small = Math.min(end, end + sign * 9);
      this.outline([center - 13, small, 26, 9], paint);
      const back = end - sign * 7;
      this.place(
        [
          [center - 8.5, end],
          [center + 8.5, end],
          [center - 8.5, back],
          [center + 8.5, back],
        ],
        () => this.goal(center, end, back),
      );
    }
  }

  private goal(center: number, end: number, back: number): void {
    const FRAME = 0xa8a4a1;
    const mesh = this.accent(0x53605b, 0x818982);
    for (const x of [center - 8.5, center + 8.5]) {
      this.rail([x, 0, back], [x, 8, end], 0.44, mesh);
      this.rail([x, 0, end], [x, 0, back], 0.5, mesh);
      this.rail([x, 0, end], [x, 8, end], 0.8, FRAME);
    }
    for (const t of [0.28, 0.56, 0.83]) {
      const z = end + (back - end) * t;
      this.rail([center - 8.5, 8 * (1 - t), z], [center + 8.5, 8 * (1 - t), z], 0.35, mesh);
    }
    for (const x of [center - 4.2, center, center + 4.2]) {
      this.rail([x, 8, end], [x, 0, back], 0.35, mesh);
    }
    this.rail([center - 8.5, 8, end], [center + 8.5, 8, end], 0.84, FRAME);
  }

  court([x, z, w, d]: Rect4): void {
    const ASPHALT = 0x464557;
    const paint = this.accent(0x645f70, 0x958689);
    const center = x + w * 0.5;
    const baseline = z + d;
    const foul = baseline - 37;
    this.patch([x - 5, z - 5, w + 10, d + 10], 0x303442);
    this.patch([x, z, w, d], ASPHALT);
    this.patch([center - 13.5, foul, 27, 37], this.accent(0x504752, 0x62505b));
    this.outline([x, z, w, d], paint);
    this.outline([center - 13.5, foul, 27, 37], paint);
    this.arc([center, foul], 13.5, [Math.PI, 2 * Math.PI], paint);
    this.arc([center, baseline - 10], 42, [Math.PI, 2 * Math.PI], paint);
    for (const side of [-1, 1]) {
      this.patch([center + side * 42 - 0.42, baseline - 10, 0.85, 10], paint);
      for (const offset of [7, 14, 21]) {
        const mark = center + side * 13.5;
        this.patch([Math.min(mark, mark + side * 2.6), foul + offset, 2.6, 0.85], paint);
      }
    }
    this.place([[center, baseline + 5]], () => this.hoop(center, baseline));
  }

  private hoop(x: number, baseline: number): void {
    const pole = baseline + 5;
    const board = baseline - 4;
    const rim = board - 4.3;
    this.rail([x, 0, pole], [x, 25, pole], 1.4, 0x827a88);
    this.rail([x, 25, pole], [x, 25, board], 1, 0x797381);
    const rect = (w: number, bottom: number, top: number, z: number): Point3[] => [
      [x - w, bottom, z],
      [x + w, bottom, z],
      [x + w, top, z],
      [x - w, top, z],
    ];
    this.fixture(rect(7.9, 21, 29.8, board + 0.8), 0x5b5769);
    this.fixture(rect(7.5, 21.2, 29.45, board), this.accent(0x8b828f, 0xa59ba1));
    this.fixture(rect(3, 22.25, 26.25, board - 0.05), 0xc6b6b4);
    this.fixture(rect(2.5, 22.78, 25.71, board - 0.1), 0x8e8490);
    const ring = (i: number, radius: number, y: number): Point3 => {
      const angle = (i * Math.PI * 2) / 16;
      return [x + radius * Math.cos(angle), y, rim + radius * Math.sin(angle)];
    };
    for (let i = 0; i < 16; i++) {
      this.rail(ring(i, 3.6, 22), ring(i + 1, 3.6, 22), 0.72, 0xad7e72);
      if (i % 4 === 0) {
        this.rail(ring(i, 3.4, 22), ring(i + 1, 2.4, 17.5), 0.38, 0x998e94);
      }
    }
  }
}

export class Bank {
  readonly view: View;
  readonly edge: readonly Point2[];

  constructor(view: View) {
    this.view = view;
    const w = view.width;
    this.edge = [
      [-420, 152],
      [w * 0.12, 152],
      [w * 0.12, 138],
      [w * 0.25, 138],
      [w * 0.25, 155],
      [w * 0.43, 155],
      [w * 0.43, 133],
      [w * 0.65, 133],
      [w * 0.65, 163],
      [w * 0.77, 163],
      [w * 0.77, 147],
      [w + 315, 147],
    ];
  }

  water(height: number): ArtPath[] {
    const paths: ArtPath[] = [];
    const rim = this.edge.map(([x, z]) => this.view.onWater(x, z));
    const last = rim[rim.length - 1]!;
    const first = rim[0]!;
    pushShape(paths, [...rim, [last[0], height], [first[0], height]], WATER, 1);
    return paths;
  }

  inland(): ArtPath[] {
    const paths: ArtPath[] = [];
    const y = streetGrade(this.view);
    const at = (x: number, depth: number): Point3 => [x, y, depth];
    const pavement = STREET_DEPTH - 19;
    const curb = STREET_DEPTH - 6;
    const left = this.edge[0]![0];
    const right = this.edge[this.edge.length - 1]![0];
    const edgePts: Point3[] = this.edge.map(([x, z]) => [
      x,
      this.view.waterline() + WALL_TOP,
      z + 52,
    ]);
    this.view.face(paths, [...edgePts, at(right, pavement), at(left, pavement)], 0x222639, 1);
    for (const [[near, far], color] of [
      [[pavement, curb], 0x1d2233],
      [[curb, STREET_DEPTH + 4], 0x303043],
    ] as const) {
      this.view.face(
        paths,
        [at(left, near), at(right, near), at(right, far), at(left, far)],
        color,
        1,
      );
    }
    this.view.line(paths, at(left, curb), at(right, curb), 0.7, 0x605b70, 0.46);
    return paths;
  }

  shore(): ArtPath[] {
    const paths: ArtPath[] = [];
    const y = this.view.waterline();
    const at = ([x, z]: Point2, dy: number): Point3 => [x, y + dy, z];
    const apron = this.edge.map((p) => at(p, WALL_TOP));
    const inland = [...this.edge].reverse().map(([x, z]) => at([x, z + 52], WALL_TOP));
    this.view.face(paths, [...apron, ...inland], 0x25293b, 1);

    for (let idx = 0; idx < this.edge.length - 1; idx++) {
      const a = this.edge[idx]!;
      const b = this.edge[idx + 1]!;
      const alongBank = a[0] !== b[0];
      const stone = alongBank ? 0x38384b : 0x2d3144;
      this.view.face(paths, [at(a, WALL_TOP), at(b, WALL_TOP), at(b, 0), at(a, 0)], stone, 1);
      this.view.line(paths, at(a, WALL_TOP), at(b, WALL_TOP), 1.05, 0x736b79, 0.46);
      this.view.line(paths, at(a, -1.4), at(b, -1.4), 0.9, 0x151e31, 0.68);
      if (alongBank) {
        const left = Math.max(a[0], -140);
        const right = Math.min(b[0], this.view.width + 75);
        const count = Math.floor((right - left) / 49);
        for (let i = 0; i < count; i++) {
          const x = left + 15 + i * 49;
          this.view.line(
            paths,
            at([x - 1, a[1]], WALL_TOP - 1.4),
            at([x - 1, a[1]], -4),
            1.65,
            0x4c495c,
            1,
          );
        }
      }
    }

    for (const [fraction, length] of [
      [0.31, 28],
      [0.56, 25],
      [0.86, 32],
    ] as const) {
      const x = this.view.width * fraction;
      let bankIdx = 0;
      while (bankIdx < this.edge.length && this.edge[bankIdx]![0] <= x) bankIdx += 1;
      const bank = this.edge[Math.max(0, bankIdx - 1)]![1];
      const tip = bank - length;
      const a: Point2 = [x, bank];
      const b: Point2 = [x + 18, bank];
      const c: Point2 = [x + 18, tip];
      const d: Point2 = [x, tip];
      this.view.face(
        paths,
        [at(a, 0), at([x + 21, bank], 0), at([x + 21, tip - 3], 0), at([x - 2, tip - 3], 0)],
        0x070f20,
        0.35,
      );
      this.view.face(paths, [at(d, DOCK_TOP), at(c, DOCK_TOP), at(c, -1), at(d, -1)], 0x333447, 1);
      this.view.face(paths, [at(b, DOCK_TOP), at(c, DOCK_TOP), at(c, -1), at(b, -1)], 0x272d40, 1);
      this.view.face(
        paths,
        [at(a, DOCK_TOP), at(b, DOCK_TOP), at(c, DOCK_TOP), at(d, DOCK_TOP)],
        0x454354,
        1,
      );
      for (const frac of [0.35, 0.7]) {
        const row = bank - length * frac;
        this.view.line(
          paths,
          at([x, row], DOCK_TOP),
          at([x + 18, row], DOCK_TOP),
          0.55,
          0x2a2d40,
          0.68,
        );
      }
      for (const pt of [c, d, [x + 18, bank - length * 0.53] as Point2]) {
        this.view.line(paths, at(pt, DOCK_TOP - 1.9), at(pt, 0.6), 1.3, 0x655a6a, 0.82);
      }
    }
    return paths;
  }
}
