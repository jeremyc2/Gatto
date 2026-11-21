import { remEuclid, rgbCss, type Point2, type Point3, type Rect4 } from "../types";
import { Camera, View } from "./projection";

type VehicleKind = "car" | "truck" | "semi" | "bus";
type EndKind =
  | "plain"
  | "grille"
  | "tailgate"
  | "windshield"
  | "rearGlass"
  | "cargoDoors"
  | "busFront"
  | "busRear";

function vehicleWidth(kind: VehicleKind): number {
  switch (kind) {
    case "car":
      return 15;
    case "truck":
      return 20;
    case "semi":
      return 33;
    case "bus":
      return 27;
  }
}

function vehicleHalfTrack(kind: VehicleKind): number {
  switch (kind) {
    case "car":
      return 2.7;
    case "truck":
      return 3.3;
    case "semi":
    case "bus":
      return 3.9;
  }
}

function paintEnd(kind: EndKind, panel: (rect: Rect4, ink: number) => void): void {
  const glass = 0x7a8fa9;
  const dark = 0x403844;
  switch (kind) {
    case "plain":
      return;
    case "windshield":
    case "rearGlass":
      panel([0.13, 0.17, 0.74, 0.69], 0x424858);
      panel([0.18, 0.23, 0.64, 0.5], glass);
      panel([0.2, 0.25, 0.59, 0.12], 0x99a9bd);
      return;
    case "grille":
    case "busFront": {
      let grilleY = 0.23;
      if (kind === "busFront") {
        panel([0.13, 0.08, 0.74, 0.11], dark);
        panel([0.29, 0.105, 0.42, 0.048], 0xd2a672);
        panel([0.12, 0.24, 0.76, 0.35], 0x4b4e60);
        panel([0.16, 0.27, 0.31, 0.28], glass);
        panel([0.53, 0.27, 0.31, 0.28], glass);
        grilleY = 0.66;
      }
      panel([0.28, grilleY, 0.44, 0.78 - grilleY], dark);
      panel([0.32, grilleY + 0.06, 0.36, 0.045], 0x80727b);
      panel([0.075, 0.63, 0.18, 0.16], 0xffdd9f);
      panel([0.745, 0.63, 0.18, 0.16], 0xffdd9f);
      panel([0.03, 0.85, 0.94, 0.115], 0x9e9294);
      return;
    }
    case "tailgate":
    case "cargoDoors":
    case "busRear":
      if (kind === "cargoDoors") {
        panel([0.11, 0.1, 0.78, 0.65], 0x655c64);
        for (const u of [0.135, 0.51]) {
          panel([u, 0.135, 0.355, 0.58], 0x9a8c87);
          panel([u + 0.08, 0.16, 0.03, 0.51], 0x655c64);
          panel([u + 0.08, 0.53, 0.12, 0.055], 0xb6a79c);
        }
      } else if (kind === "busRear") {
        panel([0.17, 0.15, 0.66, 0.31], dark);
        panel([0.22, 0.18, 0.56, 0.24], glass);
        panel([0.29, 0.59, 0.42, 0.05], 0x9d7d69);
      } else {
        panel([0.21, 0.23, 0.58, 0.05], dark);
        panel([0.42, 0.34, 0.16, 0.09], 0xa29699);
      }
      panel([0.076, 0.68, 0.15, 0.14], 0xd88280);
      panel([0.774, 0.68, 0.15, 0.14], 0xd88280);
      panel([0.035, 0.87, 0.93, 0.095], dark);
      panel([0.42, 0.8, 0.16, 0.105], 0xb3a5a2);
      return;
  }
}

class VehiclePainter {
  private readonly ctx: CanvasRenderingContext2D;
  private readonly view: View;
  private readonly scale: number;
  private readonly start: number;
  private readonly direction: number;
  private readonly depth: number;
  private readonly halfTrack: number;
  readonly nearSide: number;
  readonly eyeX: number;
  private readonly eyeY: number;
  private readonly roadY: number;

  constructor(
    ctx: CanvasRenderingContext2D,
    view: View,
    scale: number,
    start: number,
    direction: number,
    depth: number,
    halfTrack: number,
    nearSide: number,
    eyeX: number,
    eyeY: number,
    roadY: number,
  ) {
    this.ctx = ctx;
    this.view = view;
    this.scale = scale;
    this.start = start;
    this.direction = direction;
    this.depth = depth;
    this.halfTrack = halfTrack;
    this.nearSide = nearSide;
    this.eyeX = eyeX;
    this.eyeY = eyeY;
    this.roadY = roadY;
  }

  face(points: readonly Point3[], ink: number): void {
    const poly = this.view.polygon(
      points.map(([x, z, y]) => [this.start + x * this.direction, this.roadY + y, this.depth + z]),
    );
    if (poly.length < 3) return;
    const ctx = this.ctx;
    const s = this.scale;
    ctx.fillStyle = rgbCss(ink);
    ctx.beginPath();
    ctx.moveTo(poly[0]![0] * s, poly[0]![1] * s);
    for (let i = 1; i < poly.length; i++) {
      ctx.lineTo(poly[i]![0] * s, poly[i]![1] * s);
    }
    ctx.closePath();
    ctx.fill();
  }

  side([x, y, w, h]: Rect4, ink: number): void {
    const z = this.nearSide;
    this.face(
      [
        [x, z, y],
        [x + w, z, y],
        [x + w, z, y + h],
        [x, z, y + h],
      ],
      ink,
    );
  }

  body(
    [left, right]: Point2,
    [top, bottom]: Point2,
    [side, roof, end]: readonly [number, number, number],
    [rear, front]: readonly [EndKind, EndKind],
  ): void {
    const z = this.halfTrack;
    if (this.eyeY < this.roadY + top) {
      this.face(
        [
          [left, -z, top],
          [right, -z, top],
          [right, z, top],
          [left, z, top],
        ],
        roof,
      );
    }
    this.side([left, top, right - left, bottom - top], side);
    let x: number;
    let detail: EndKind;
    if (this.eyeX < left) {
      x = left;
      detail = rear;
    } else if (this.eyeX > right) {
      x = right;
      detail = front;
    } else {
      return;
    }
    this.face(
      [
        [x, -z, top],
        [x, z, top],
        [x, z, bottom],
        [x, -z, bottom],
      ],
      end,
    );
    paintEnd(detail, ([u, v, w, h], ink) => {
      const a = z * (2 * u - 1);
      const b = a + 2 * z * w;
      const y = top + v * (bottom - top);
      const foot = y + h * (bottom - top);
      this.face(
        [
          [x, a, y],
          [x, b, y],
          [x, b, foot],
          [x, a, foot],
        ],
        ink,
      );
    });
  }

  roof([left, right]: Point2, y: number, ink: number): void {
    if (this.eyeY < this.roadY + y) {
      const z = this.halfTrack * 0.73;
      this.face(
        [
          [left, -z, y],
          [right, -z, y],
          [right, z, y],
          [left, z, y],
        ],
        ink,
      );
    }
  }

  wheel(axle: number, z: number, showHub: boolean): void {
    const radius = 1.42;
    const center = axle + radius;
    const circle = (r: number): Point3[] => {
      const pts: Point3[] = [];
      for (let i = 0; i < 16; i++) {
        const angle = (i * Math.PI * 2) / 16;
        pts.push([center + r * Math.sin(angle), z, -radius + r * Math.cos(angle)]);
      }
      return pts;
    };
    this.face(circle(radius), 0x1d1b2b);
    if (showHub) {
      this.face(circle(0.61), 0x777486);
    }
  }
}

function paintVehicle(kind: VehicleKind, painter: VehiclePainter): void {
  const width = vehicleWidth(kind);
  const axles = kind === "semi" ? [5, 10, 24, 29] : [3, width - 5.2];
  for (const axle of axles) painter.wheel(axle, -painter.nearSide, false);

  const glass = 0x8196b0;
  if (kind === "car") {
    const paint = [0x986a77, 0xba8990, 0x795361] as const;
    painter.body([0, width], [-4.7, -1.25], paint, ["tailgate", "grille"]);
    painter.body([3.2, 11.3], [-7.1, -4.7], paint, ["rearGlass", "windshield"]);
    painter.side([4.3, -6.65, 5.7, 1.65], glass);
    painter.side([7.3, -6.65, 0.5, 4.55], 0x785966);
    painter.side([8.05, -4.15, 1, 0.42], 0xc99a9c);
  } else if (kind === "truck") {
    const paint = [0x667b90, 0x8197a9, 0x4c5c73] as const;
    painter.body([0, width], [-5.5, -1.2], paint, ["tailgate", "grille"]);
    painter.roof([0.9, 9.7], -5.5, 0x45576c);
    painter.body([10.1, 17], [-8.7, -5.5], paint, ["rearGlass", "windshield"]);
    painter.side([11.7, -8.05, 3.9, 2.25], glass);
    painter.side([11.35, -5.5, 0.43, 3.4], 0x4c5e73);
    painter.side([12.1, -4.8, 1.35, 0.5], 0x9aaab7);
  } else if (kind === "semi") {
    painter.body([4, 30], [-3, -1.1], [0x644b55, 0x766069, 0x4c384a], ["plain", "plain"]);
    const drawTrailer = () => {
      painter.body(
        [0, 22.1],
        [-10.2, -2.1],
        [0xaca198, 0xc5b4a6, 0x837b7f],
        ["cargoDoors", "plain"],
      );
      painter.side([0.5, -9.7, 21.1, 0.6], 0xc4b6a8);
      painter.side([0.55, -3.55, 21, 0.8], 0x82777b);
      for (const x of [5.5, 11, 16.5]) painter.side([x, -8.75, 0.26, 4.6], 0x9d918b);
    };
    const drawCab = () => {
      const paint = [0x88636f, 0xb18187, 0x6a4e63] as const;
      painter.body([23, width], [-5.5, -1.2], paint, ["plain", "grille"]);
      painter.body([23, 30.4], [-8.5, -5.5], paint, ["rearGlass", "windshield"]);
      painter.side([25.8, -8, 3.4, 2.15], glass);
      painter.side([25.5, -5.5, 0.39, 3.1], 0x694b5b);
    };
    if (painter.eyeX > 22.1) {
      drawTrailer();
      drawCab();
    } else {
      drawCab();
      drawTrailer();
    }
  } else {
    painter.body([0, width], [-9.3, -1.3], [0xa28065, 0xc0a283, 0x795e57], ["busRear", "busFront"]);
    painter.side([0, -3.4, width, 1], 0xc59d7b);
    for (let i = 0; i < 5; i++) {
      const x = 1.4 + i * 3.8;
      painter.side([x, -7.85, 2.95, 2.65], glass);
      painter.side([x + 0.25, -7.6, 2.45, 0.42], 0xa5b2c2);
    }
    painter.side([20.5, -8.05, 3.7, 6.55], 0x79635b);
    painter.side([20.85, -7.75, 1.3, 3.2], glass);
    painter.side([22.53, -7.75, 1.3, 3.2], glass);
    painter.side([24.8, -7.85, 1.35, 2.65], glass);
  }

  for (const axle of axles) painter.wheel(axle, painter.nearSide, true);
  painter.side([0.05, -3.8, 1.2, 1.1], 0xd88280);
  painter.side([width - 1.3, -3.8, 1.25, 1.1], 0xffdb9b);
}

const LANES: readonly (readonly [VehicleKind, number][])[] = [
  [
    ["car", 0.08],
    ["semi", 0.26],
    ["car", 0.43],
    ["truck", 0.68],
    ["bus", 0.88],
  ],
  [
    ["bus", 0.16],
    ["car", 0.31],
    ["truck", 0.57],
    ["car", 0.74],
    ["semi", 0.94],
  ],
];

export function drawTrafficLane(
  ctx: CanvasRenderingContext2D,
  view: View,
  roadY: number,
  time: number,
  depth: number,
  scale: number,
  westbound: boolean,
): void {
  const PAD = 280;
  const length = view.width + PAD * 2;
  const speed = westbound ? -31 : 38;
  const direction = Math.sign(speed);
  const eye = (view.camera ?? Camera.atMenu(view)).eye;
  const lane = LANES[westbound ? 0 : 1]!;
  ctx.globalAlpha = 1;

  for (const [vehicle, departure] of lane) {
    const x = remEuclid(departure * length + time * speed, length) - PAD;
    const halfWidth = vehicleWidth(vehicle) * 0.5;
    const start = x - halfWidth * direction;
    const halfTrack = vehicleHalfTrack(vehicle);
    const painter = new VehiclePainter(
      ctx,
      view,
      scale,
      start,
      direction,
      depth,
      halfTrack,
      eye[2] < depth ? -halfTrack : halfTrack,
      (eye[0] - start) * direction,
      eye[1],
      roadY,
    );
    paintVehicle(vehicle, painter);
  }
}
