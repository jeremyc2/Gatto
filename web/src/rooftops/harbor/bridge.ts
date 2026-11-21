import { clamp, type ArtPath, type Point2, type Point3 } from "../types";
import {
  compileArtMesh,
  drawCompiledMesh,
  LIGHT,
  mix3,
  View,
  type CompiledLayer,
} from "./projection";
import { drawTrafficLane } from "./traffic";

const STONE = 0x504956;
const STONE_SIDE = 0x3f3c50;
const STONE_EDGE = 0x90818a;
const DECK = 0x413d50;
const STEEL = 0x746778;
const NEAR_PIER: Point2 = [-43, -29];
const CENTER_PIER: Point2 = [-7, 7];
const FAR_PIER: Point2 = [29, 43];
const OUTER_CABLE = 41;
const TOWER_HALF_DEPTH = 10;
const OVERHANG = 130;
const FOOTING: Point3 = [-23, 23, 48];
const SEAT: Point3 = [-TOWER_HALF_DEPTH, TOWER_HALF_DEPTH, FAR_PIER[1]];
const SHOULDER: Point3 = [-14.5, 14.5, 44.8];
const SEAT_INK = 0x454050;
const PLINTH_END_DIVISIONS = 4;
const COURSE = 7.8;

type Arch = readonly Point2[];
type Roadway = "far" | "near";

function roadwayEdges(road: Roadway): Point3 {
  return road === "far"
    ? [FAR_PIER[1], FAR_PIER[0], CENTER_PIER[1]]
    : [NEAR_PIER[0], NEAR_PIER[1], CENTER_PIER[0]];
}

function roadwayLaneDepth(road: Roadway): number {
  const [, jamb, median] = roadwayEdges(road);
  return (jamb + median) * 0.5;
}

function archCut(arch: Arch, y: number): Point2 {
  const center = arch[3]![0];
  let side = center;
  for (let i = 0; i < 3; i++) {
    const [z0, y0] = arch[i]!;
    const [z1, y1] = arch[i + 1]!;
    if (y0 >= y && y >= y1) {
      side = z0 + ((z1 - z0) * (y0 - y)) / (y0 - y1);
      break;
    }
  }
  return [side, 2 * center - side];
}

function towersFor(width: number): Point2 {
  return [width * 0.245 - TOWER_HALF_DEPTH, width * 0.761 - TOWER_HALF_DEPTH];
}

export function tugChannel(view: View): Point2 {
  const [west, east] = towersFor(view.width);
  return [west + FOOTING[1] + 68, east + FOOTING[0] - 65];
}

function viewCacheKey(view: View, deck: number): string {
  const c = view.camera;
  if (!c) {
    return `menu:${view.width}:${view.horizon}:${view.aim.x}:${view.aim.y}:${deck}`;
  }
  return `cam:${c.eye[0]}:${c.eye[1]}:${c.eye[2]}:${c.axes[1][1]}:${c.axes[2][2]}:${deck}`;
}

export class BridgeArt {
  private lastKey = "";
  private layers: [CompiledLayer[], CompiledLayer[], CompiledLayer[], CompiledLayer[]] = [
    [],
    [],
    [],
    [],
  ];

  render(
    ctx: CanvasRenderingContext2D,
    view: View,
    deck: number,
    time: number,
    scale: number,
  ): void {
    const key = viewCacheKey(view, deck);
    if (key !== this.lastKey) {
      this.lastKey = key;
      const built = new Bridge(
        view,
        [-OVERHANG, view.width + OVERHANG],
        towersFor(view.width),
        deck,
      ).layers();
      this.layers = [
        compileArtMesh(built[0]),
        compileArtMesh(built[1]),
        compileArtMesh(built[2]),
        compileArtMesh(built[3]),
      ];
    }

    drawCompiledMesh(ctx, this.layers[0], scale);
    drawTrafficLane(ctx, view, deck, time, roadwayLaneDepth("far"), scale, true);
    drawCompiledMesh(ctx, this.layers[1], scale);
    drawTrafficLane(ctx, view, deck, time, roadwayLaneDepth("near"), scale, false);
    drawCompiledMesh(ctx, this.layers[2], scale);
    drawCompiledMesh(ctx, this.layers[3], scale);
  }
}

class Bridge {
  private readonly view: View;
  private readonly ends: Point2;
  private readonly towers: Point2;
  private readonly deck: number;

  constructor(view: View, ends: Point2, towers: Point2, deck: number) {
    this.view = view;
    this.ends = ends;
    this.towers = towers;
    this.deck = deck;
  }

  private top(): number {
    return this.deck - 158;
  }

  private spring(): number {
    return this.top() + 63;
  }

  private seat(): number {
    return this.deck + 16;
  }

  private shoulder(): number {
    return this.deck + 24;
  }

  private courseRows(top: number, bottom: number): number[] {
    const start = this.top() + 9;
    const first = Math.max(0, Math.ceil((top - start) / COURSE));
    const rows: number[] = [];
    for (let row = first; ; row++) {
      const y = start + row * COURSE;
      if (y >= bottom) break;
      rows.push(y);
    }
    return rows;
  }

  private spans(): [Point2, Point2, Point2] {
    const [left, right] = this.ends;
    const [west, east] = this.towers;
    return [
      [left, west - TOWER_HALF_DEPTH],
      [west + TOWER_HALF_DEPTH, east - TOWER_HALF_DEPTH],
      [east + TOWER_HALF_DEPTH, right],
    ];
  }

  layers(): [ArtPath[], ArtPath[], ArtPath[], ArtPath[]] {
    const far: ArtPath[] = [];
    const middle: ArtPath[] = [];
    const near: ArtPath[] = [];
    const exposed: ArtPath[] = [];

    for (const tower of this.towers) {
      this.foundation(far, tower);
      this.deckEnd(far, tower + TOWER_HALF_DEPTH);
    }
    this.suspension(far, exposed, OUTER_CABLE, 0x85778b, 0.5);
    this.suspension(far, exposed, 4.5, 0x8c7c8d, 0.46);
    for (const tower of this.towers) {
      this.masonry(far, tower, FAR_PIER, 0x494451);
    }
    this.road(far, "far");

    this.promenadeBypasses(middle, 1);
    this.promenade(middle);
    this.suspension(middle, exposed, -4.5, 0x9e8999, 0.52);
    for (const tower of this.towers) {
      this.masonry(middle, tower, CENTER_PIER, 0x4e4755);
    }
    this.road(middle, "near");

    if (this.view.camera && this.view.camera.eye[1] > this.seat()) {
      this.soffit(near);
    }
    this.truss(near);
    this.suspension(near, exposed, -OUTER_CABLE, 0xb39ba7, 0.68);
    for (const tower of this.towers) {
      this.nearBearingFace(near, tower);
      this.masonry(near, tower, NEAR_PIER, STONE);
      this.crown(near, tower);
    }

    return [far, middle, near, exposed];
  }

  private deckEnd(paths: ArtPath[], x: number): void {
    const front = NEAR_PIER[0];
    const back = FAR_PIER[1];
    this.view.face(
      paths,
      [
        [x, this.deck, front],
        [x, this.deck, back],
        [x, this.seat(), back],
        [x, this.seat(), front],
      ],
      0x303346,
      1,
    );
  }

  private foundation(paths: ArtPath[], x: number): void {
    const waterline = this.view.waterline();
    const [fLeft, fRight, fHalf] = FOOTING;
    const contour = (spread: number): [Point3, Point3, Point3] => [
      [x + fLeft - spread * 0.33, waterline, -fHalf - spread],
      [x + fRight + spread * 0.58, waterline, -fHalf - spread],
      [x + fRight + spread * 0.58, waterline, fHalf + spread * 0.2],
    ];
    let inner = contour(0);
    for (const [spread, alpha] of [
      [2.3, 0.47],
      [5, 0.29],
      [8.7, 0.16],
      [14, 0.075],
    ] as const) {
      const outer = contour(spread);
      for (let side = 0; side < 2; side++) {
        this.view.face(
          paths,
          [inner[side]!, inner[side + 1]!, outer[side + 1]!, outer[side]!],
          0x070f20,
          alpha,
        );
      }
      inner = outer;
    }

    const top = this.seat();
    const base = this.shoulder();
    const [sLeft, sRight, sHalf] = SEAT;
    this.view.face(
      paths,
      [
        [x + sLeft, top, -sHalf],
        [x + sRight, top, -sHalf],
        [x + sRight, top, sHalf],
        [x + sLeft, top, sHalf],
      ],
      0x645c6b,
      1,
    );

    const stem: Point3 = [-18.5, 18.5, 46.5];
    this.plinth(paths, x, [top, base], [SEAT, SHOULDER], [0x57515e, SEAT_INK]);
    this.plinth(paths, x, [base, base + 4], [SHOULDER, stem], [0x615765, 0x4f4657]);
    const wetHeight = 3.4;
    const wetFrac = wetHeight / (waterline - (base + 4));
    const wetEdge: Point3 = [
      FOOTING[0] + (stem[0] - FOOTING[0]) * wetFrac,
      FOOTING[1] + (stem[1] - FOOTING[1]) * wetFrac,
      FOOTING[2] + (stem[2] - FOOTING[2]) * wetFrac,
    ];
    this.plinth(paths, x, [base + 4, waterline - wetHeight], [stem, wetEdge], [0x504b5a, 0x3b394b]);
    this.plinth(
      paths,
      x,
      [waterline - wetHeight, waterline],
      [wetEdge, FOOTING],
      [0x454453, 0x343749],
    );
    const [left, corner, right] = contour(0);
    this.view.line(paths, left, corner, 0.85, 0x778198, 0.27);
    this.view.line(paths, corner, right, 0.7, 0x758196, 0.2);
  }

  private plinthEdge(x: number, y: number, [left, right, half]: Point3): [Point3, Point3, Point3] {
    return [
      [x + left, y, -half],
      [x + right, y, -half],
      [x + right, y, half],
    ];
  }

  private nearBearingFace(paths: ArtPath[], x: number): void {
    const edge = (t: number): [Point3, Point3] => {
      const flare = (SHOULDER[2] - SEAT[2]) * t;
      const face = x + SEAT[1] + (SHOULDER[1] - SEAT[1]) * t;
      const y = this.seat() + (this.shoulder() - this.seat()) * t;
      return [
        [face, y, NEAR_PIER[0] - flare],
        [face, y, NEAR_PIER[1] + flare],
      ];
    };
    const [front, rear] = edge(0);
    const [footFront, footRear] = edge(1);
    const [midFront, midRear] = edge(0.5);
    this.view.face(paths, [front, rear, footRear, footFront], SEAT_INK, 1);
    this.view.line(paths, midFront, midRear, 0.65, 0x242535, 0.67);
    this.view.line(paths, front, footFront, 1.25, STONE_EDGE, 0.31);
  }

  private plinth(
    paths: ArtPath[],
    x: number,
    [top, bottom]: Point2,
    [upper, lower]: [Point3, Point3],
    [front, end]: Point2,
  ): void {
    const edge = (y: number): [Point3, Point3, Point3] => {
      const t = (y - top) / (bottom - top);
      return this.plinthEdge(x, y, [
        upper[0] + (lower[0] - upper[0]) * t,
        upper[1] + (lower[1] - upper[1]) * t,
        upper[2] + (lower[2] - upper[2]) * t,
      ]);
    };
    const a = edge(top);
    const b = edge(bottom);
    this.view.face(paths, [a[0], a[1], b[1], b[0]], front, 1);
    this.view.face(paths, [a[1], a[2], b[2], b[1]], end, 1);
    const courses = Math.floor((bottom - top) / 7.4);
    const pitch = (bottom - top) / (courses + 1);
    for (let course = 0; course <= courses; course++) {
      const y = top + pitch * course;
      const row = edge(y);
      const below = edge(y + pitch);
      if (course > 0) {
        this.view.line(paths, row[0], row[1], 0.65, 0x252636, 0.64);
        this.view.line(paths, row[1], row[2], 0.65, 0x242535, 0.67);
      }
      if (courses > 0) {
        for (const [side, divisions] of [
          [0, 2],
          [1, PLINTH_END_DIVISIONS],
        ] as const) {
          for (let joint = 1; joint <= divisions; joint++) {
            const f = (joint - (course % 2) * 0.48) / (divisions + 1);
            const join = (line: [Point3, Point3, Point3]): Point3 =>
              mix3(line[side]!, line[side + 1]!, f);
            this.view.line(paths, join(row), join(below), 0.5, 0x242535, 0.34);
          }
        }
      }
    }
    this.view.line(paths, a[1], b[1], 1.25, STONE_EDGE, 0.31);
  }

  private masonry(paths: ArtPath[], x: number, [a, b]: Point2, ink: number): void {
    const top = this.spring();
    const bottom = this.seat();
    const front = x + TOWER_HALF_DEPTH;
    const at = (z: number, y: number): Point3 => [front, y, z];
    const outer = a === NEAR_PIER[0];
    const back = (z: number, y: number): Point3 => [x - TOWER_HALF_DEPTH, y, z];

    this.view.face(
      paths,
      [back(a, top), at(a, top), at(a, bottom), back(a, bottom)],
      outer ? STONE_SIDE : 0x2d2e40,
      1,
    );
    this.view.face(paths, [at(a, top), at(b, top), at(b, bottom), at(a, bottom)], ink, 1);
    this.view.line(paths, at(a, top), at(a, bottom), 0.95, STONE_EDGE, 0.31);

    const rows = this.courseRows(top, bottom);
    for (const y of rows) {
      this.view.line(paths, at(a, y), at(b, y), 0.65, STONE_SIDE, 0.72);
    }
    for (const y of rows) {
      this.view.line(paths, at(a, y), back(a, y), 0.55, 0x211f32, 0.53);
    }
    const jointRows = this.courseRows(top - 0.001, bottom - 2);
    for (let row = 0; row < jointRows.length; row++) {
      const y = jointRows[row]!;
      const joint = row % 2 === 0 ? a + 4 : b - 4;
      this.view.line(
        paths,
        at(joint, y),
        at(joint, Math.min(y + COURSE, bottom)),
        0.48,
        0x272838,
        0.31,
      );
    }
  }

  private crown(paths: ArtPath[], x: number): void {
    const front = x + TOWER_HALF_DEPTH;
    const at = (z: number, y: number): Point3 => [front, y, z];
    const back = (z: number, y: number): Point3 => [x - TOWER_HALF_DEPTH, y, z];
    const top = this.top();
    const spring = this.spring();
    const opening = (a: number, b: number): Arch => {
      const width = b - a;
      return [
        [a, spring],
        [a + width * 0.07, spring - 11],
        [a + width * 0.22, spring - 23],
        [(a + b) * 0.5, spring - 33],
        [b - width * 0.22, spring - 23],
        [b - width * 0.07, spring - 11],
        [b, spring],
      ];
    };
    const nearArch = opening(NEAR_PIER[1], CENTER_PIER[0]);
    const farArch = opening(CENTER_PIER[1], FAR_PIER[0]);

    this.view.face(
      paths,
      [
        at(NEAR_PIER[0], top),
        at(FAR_PIER[1], top),
        back(FAR_PIER[1], top),
        back(NEAR_PIER[0], top),
      ],
      0x726278,
      1,
    );
    const firstJambCourse = this.courseRows(spring, spring + 10)[0] ?? spring + 7;
    for (const arch of [farArch, nearArch]) {
      for (let i = 3; i < arch.length - 1; i++) {
        const e0 = arch[i]!;
        const e1 = arch[i + 1]!;
        this.view.face(
          paths,
          [at(e0[0], e0[1]), at(e1[0], e1[1]), back(e1[0], e1[1]), back(e0[0], e0[1])],
          0x2d2e40,
          1,
        );
      }
    }
    this.view.face(
      paths,
      [
        back(NEAR_PIER[0], top),
        at(NEAR_PIER[0], top),
        at(NEAR_PIER[0], spring),
        back(NEAR_PIER[0], spring),
      ],
      STONE_SIDE,
      1,
    );
    const outline: Point3[] = [
      at(NEAR_PIER[0], top),
      at(FAR_PIER[1], top),
      at(FAR_PIER[1], spring),
    ];
    for (let i = farArch.length - 1; i >= 0; i--) {
      outline.push(at(farArch[i]![0], farArch[i]![1]));
    }
    for (let i = nearArch.length - 1; i >= 0; i--) {
      outline.push(at(nearArch[i]![0], nearArch[i]![1]));
    }
    outline.push(at(NEAR_PIER[0], spring));
    this.view.face(paths, outline, STONE, 1);

    const west = NEAR_PIER[0];
    const east = FAR_PIER[1];
    const rows = this.courseRows(top, spring);
    for (let row = 0; row < rows.length; row++) {
      const y = rows[row]!;
      const end = Math.min(y + COURSE, spring);
      const near = archCut(nearArch, y);
      const far = archCut(farArch, y);
      for (const [a, b] of [
        [west, near[0]],
        [near[1], far[0]],
        [far[1], east],
      ] as const) {
        this.view.line(paths, at(a, y), at(b, y), 0.65, STONE_SIDE, 0.67);
      }
      const nearEnd = archCut(nearArch, end);
      const farEnd = archCut(farArch, end);
      const stagger = (row % 2) * 5;
      for (const baseZ of [-33, -22, -11, 0, 11, 22, 33]) {
        const z = baseZ + stagger;
        if ((z < nearEnd[0] || z > nearEnd[1]) && (z < farEnd[0] || z > farEnd[1])) {
          this.view.line(paths, at(z, y), at(z, end), 0.5, 0x292838, 0.36);
        }
      }
    }
    for (const y of rows) {
      this.view.line(paths, back(west, y), at(west, y), 0.55, 0x211f32, 0.53);
    }
    this.view.line(paths, at(west, top), at(west, spring), 0.95, STONE_EDGE, 0.31);
    for (const [y, width, opacity] of [
      [top + 1, 3, 0.8],
      [top + 7, 1, 0.55],
      [top + 17, 1.6, 0.52],
    ] as const) {
      this.view.line(
        paths,
        at(NEAR_PIER[0] - 0.7, y),
        at(FAR_PIER[1] + 0.7, y),
        width,
        STONE_EDGE,
        opacity,
      );
    }

    for (const arch of [nearArch, farArch]) {
      const center = arch[3]![0];
      const half = center - arch[0]![0];
      const inner = arch.map(([z, y]) => at(z, y));
      const rim = arch.map(([z, y]) => {
        const side = (z - center) / half;
        return at(z + side * 3, y - (1 - Math.abs(side)) * 4);
      });
      for (let i = 0; i < 6; i++) {
        const stone = i % 2 === 0 ? 0x675c69 : 0x605563;
        this.view.face(paths, [rim[i]!, rim[i + 1]!, inner[i + 1]!, inner[i]!], stone, 1);
      }
      for (let i = 1; i < 6; i++) {
        this.view.line(paths, inner[i]!, rim[i]!, 0.55, 0x302b38, 0.8);
      }
      for (const [z, side, ink] of [
        [arch[0]![0], -1, 0x675c69],
        [arch[6]![0], 1, 0x605563],
      ] as const) {
        const block = (y: number, width: number): [Point3, Point3] => [
          at(z, y),
          at(z + side * width, y),
        ];
        const [inside, outside] = block(spring, 3);
        const [next, edge] = block(firstJambCourse, 2.5);
        const [last, foot] = block(firstJambCourse + COURSE, 1.2);
        this.view.face(paths, [inside, outside, edge, next], ink, 0.86);
        this.view.face(paths, [next, edge, foot, last], ink, 0.44);
        this.view.line(paths, next, edge, 0.47, 0x302b38, 0.65);
      }
    }

    for (const z of [-OUTER_CABLE, -4.5, 4.5, OUTER_CABLE]) {
      const saddle: Point3 = [x, top, z];
      this.view.detail(paths, saddle, [-6.7, -2.2, 13.4, 2.2], 0x846e7c, 1);
      this.view.detail(paths, saddle, [-4.4, -5.3, 8.8, 3.2], 0x9c8391, 1);
    }
  }

  private road(paths: ArtPath[], road: Roadway): void {
    const [outer, wall, inner] = roadwayEdges(road);
    const [ink, shoulder] = road === "far" ? [0x504b5b, 0x3f3a4e] : [0x51495b, DECK];
    const y = this.deck;
    const at = (x: number, z: number): Point3 => [x, y, z];
    const [left, right] = this.ends;
    for (const [start, end] of this.spans()) {
      this.view.face(
        paths,
        [at(start, outer), at(end, outer), at(end, wall), at(start, wall)],
        shoulder,
        1,
      );
      this.view.line(paths, at(start, outer), at(end, outer), 0.85, STEEL, 0.8);
    }
    this.view.face(
      paths,
      [at(left, wall), at(right, wall), at(right, inner), at(left, inner)],
      ink,
      1,
    );
    if (road === "far") {
      const lip = (x: number): Point3 => [x, y + 3, inner];
      this.view.face(
        paths,
        [at(left, inner), at(right, inner), lip(right), lip(left)],
        0x353143,
        1,
      );
      this.view.line(paths, at(left, inner), at(right, inner), 0.68, STEEL, 0.84);
    }
  }

  private soffit(paths: ArtPath[]): void {
    const front = NEAR_PIER[0];
    const rear = FAR_PIER[1];
    const [left, right] = this.ends;
    const y = this.seat();
    this.view.face(
      paths,
      [
        [left, y, front],
        [right, y, front],
        [right, y, rear],
        [left, y, rear],
      ],
      0x222536,
      1,
    );
    this.view.face(
      paths,
      [
        [left, this.deck, rear],
        [right, this.deck, rear],
        [right, y, rear],
        [left, y, rear],
      ],
      0x333044,
      1,
    );
    for (const z of [-39, -22, 0, 22, 39]) {
      this.view.line(paths, [left, y, z], [right, y, z], 1.5, 0x47435a, 1);
    }
    for (let bay = -9; bay <= 28; bay++) {
      const x = bay * 38;
      this.view.line(paths, [x, y, front], [x, y, rear], 2, 0x51475b, 0.8);
    }
  }

  private truss(paths: ArtPath[]): void {
    const at = (x: number, dy: number): Point3 => [x, this.deck + dy, NEAR_PIER[0]];
    const soffit = this.seat() - this.deck;
    const upper = 3;
    const lower = soffit - 0.7;
    for (const [left, right] of this.spans()) {
      this.view.face(
        paths,
        [at(left, 0), at(right, 0), at(right, soffit), at(left, soffit)],
        0x272b3d,
        1,
      );
      for (const chord of [0.65, upper, lower]) {
        this.view.line(paths, at(left, chord), at(right, chord), 1.5, STEEL, 0.9);
      }
      const bays = Math.ceil((right - left) / 18);
      const bayAt = (idx: number) => left + ((right - left) * idx) / bays;
      for (let bay = 0; bay < bays; bay++) {
        const a = at(bayAt(bay), upper);
        const tip = at(bayAt(bay + 0.5), lower);
        const b = at(bayAt(bay + 1), upper);
        this.view.line(paths, a, tip, 1.08, STEEL, 0.85);
        this.view.line(paths, tip, b, 1.08, STEEL, 0.85);
      }
      for (const end of [left, right]) {
        this.view.line(paths, at(end, upper), at(end, lower), 1.4, STEEL, 0.84);
      }
    }
  }

  private promenade(paths: ArtPath[]): void {
    const y = this.deck - 12;
    const [left, right] = this.ends;
    const [west, east] = this.towers;
    const margin = TOWER_HALF_DEPTH + 24;
    for (const [start, end] of [
      [left, west - margin],
      [west + margin, east - margin],
      [east + margin, right],
    ] as const) {
      this.walkway(
        paths,
        y,
        [
          [start, -8],
          [end, -8],
        ],
        [
          [start, 8],
          [end, 8],
        ],
      );
    }
    this.promenadeBypasses(paths, -1);
  }

  private promenadeBypasses(paths: ArtPath[], side: number): void {
    const y = this.deck - 12;
    const margin = TOWER_HALF_DEPTH + 24;
    const corner = TOWER_HALF_DEPTH + 5;
    for (const x of this.towers) {
      const bent = (initial: number, shifted: number): Point2[] => [
        [x - margin, initial * side],
        [x - corner, shifted * side],
        [x + corner, shifted * side],
        [x + margin, initial * side],
      ];
      this.walkway(paths, y, bent(8, 17), bent(0, 9));
    }
  }

  private walkway(paths: ArtPath[], y: number, a: readonly Point2[], b: readonly Point2[]): void {
    const at = (p: Point2): Point3 => [p[0], y, p[1]];
    this.view.face(paths, [...a.map(at), ...[...b].reverse().map(at)], 0x786575, 0.96);
    for (const edge of [a, b]) {
      for (let s = 0; s < edge.length - 1; s++) {
        const pa = at(edge[s]!);
        const pb = at(edge[s + 1]!);
        const raised = (p: Point3) => this.view.offset(p, 0, -3.5);
        this.view.line(paths, pa, pb, 1.35, 0x473b4e, 1);
        this.view.line(paths, raised(pa), raised(pb), 0.82, 0x9c818c, 0.65);
        const count = Math.ceil((pb[0] - pa[0]) / 15);
        for (let idx = 0; idx < count; idx++) {
          const post = mix3(pa, pb, idx / count);
          this.view.detail(paths, post, [0, -3.5, 0.65, 3.5], 0x8b7484, 0.73);
          if (idx === 0 && count > 5) {
            this.view.detail(paths, post, [0, -10, 0.95, 10], STEEL, 1);
            this.view.detail(paths, post, [-0.9, -10.6, 2.7, 2.2], LIGHT, 0.88);
          }
        }
      }
    }
  }

  private suspension(
    paths: ArtPath[],
    exposed: ArtPath[],
    z: number,
    ink: number,
    opacity: number,
  ): void {
    const [west, east] = this.towers;
    const [left, right] = this.ends;
    const outer = Math.abs(z) > 20;
    const rail = outer ? this.deck + 3 : this.deck - 10;
    const top = this.top() - 6;
    const rise = this.deck - 17 - top;
    const at = (x: number, y: number): Point3 => [x, y, z];
    const spans = this.spans();
    const sockets: Point3[] = [];
    const intervals: readonly Point2[] = [
      [left, west - 4.9],
      [west - 4.9, west + 4.9],
      [west + 4.9, east - 4.9],
      [east - 4.9, east + 4.9],
      [east + 4.9, right],
    ];
    const weight = outer ? 1.65 : 1.15;
    const cableRibbon = (target: ArtPath[], pts: readonly Point3[]) => {
      const ribbon: Point3[] = [...pts];
      for (let i = pts.length - 1; i >= 0; i--) {
        ribbon.push(this.view.offset(pts[i]!, 0, weight));
      }
      this.view.face(target, ribbon, ink, opacity);
    };

    for (let index = 0; index < intervals.length; index++) {
      const [start, finish] = intervals[index]!;
      const cable = (x: number): Point3 => {
        const t = clamp((x - start) / (finish - start), 0, 1);
        const sag =
          index === 0
            ? 0.91 * Math.pow(1 - t, 1.12)
            : index === 2
              ? 4 * t * (1 - t)
              : index === 4
                ? 0.91 * Math.pow(t, 1.12)
                : 0;
        return at(x, top + rise * sag);
      };
      const count = Math.ceil((finish - start) / 8);
      const pts: Point3[] = [];
      for (let i = 0; i <= count; i++) {
        pts.push(cable(start + ((finish - start) * i) / count));
      }
      if (index === 0) {
        cableRibbon(paths, pts);
      } else if (index === 2) {
        const mid = Math.floor(pts.length / 2);
        cableRibbon(exposed, pts.slice(0, mid + 1));
        cableRibbon(paths, pts.slice(mid));
      } else {
        cableRibbon(exposed, pts);
      }

      if (index % 2 === 0) {
        const [a, b] = spans[index / 2]!;
        const pitch = outer ? 19 : 29;
        const bays = Math.max(1, Math.round((b - a) / pitch));
        this.view.line(paths, at(a, rail), at(b, rail), 1.15, STEEL, outer ? 0.68 : 0.36);
        for (let i = 1; i < bays; i++) {
          const x = a + ((b - a) * i) / bays;
          const socket = at(x, rail);
          this.view.line(paths, cable(x), socket, 0.65, ink, opacity * 0.55);
          if (outer) sockets.push(socket);
        }
      }
    }

    if (outer) {
      for (const [base, ends] of [
        [west, [left, east]],
        [east, [west, right]],
      ] as const) {
        for (const end of ends) {
          const face = base + Math.sign(end - base) * TOWER_HALF_DEPTH;
          const layer = end > base ? exposed : paths;
          for (const fraction of [0.15, 0.24, 0.33]) {
            const socket = at(base + (end - base) * fraction, rail);
            this.view.line(layer, at(face, top + 5.5), socket, 0.68, ink, opacity * 0.53);
            sockets.push(socket);
          }
        }
      }
      for (const socket of sockets) {
        this.view.detail(paths, socket, [-0.9, -1, 1.8, 2.2], ink, opacity * 0.84);
      }
    }
  }
}
