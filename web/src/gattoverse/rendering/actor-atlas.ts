import type Phaser from "phaser";
import type { ActorKind } from "../simulation/contracts";

export const FRAME_SIZE = 96;
export const WALK_FRAMES = 6;
export const actorKinds: ActorKind[] = [
  "cat",
  "watercolor",
  "paper",
  "velvet",
  "rat",
  "owl",
  "unicorn",
];

function oval(
  c: CanvasRenderingContext2D,
  x: number,
  y: number,
  rx: number,
  ry: number,
  fill: string,
  angle = 0,
) {
  c.fillStyle = fill;
  c.beginPath();
  c.ellipse(x, y, rx, ry, angle, 0, Math.PI * 2);
  c.fill();
}
function shape(c: CanvasRenderingContext2D, points: number[], fill: string) {
  c.fillStyle = fill;
  c.beginPath();
  points.forEach((n, i) => {
    if (i % 2) return;
    if (!i) c.moveTo(n, points[i + 1] ?? 0);
    else c.lineTo(n, points[i + 1] ?? 0);
  });
  c.closePath();
  c.fill();
}

/** Each direction is painted from a different pose; these are not rotations of one image. */
function paintCat(c: CanvasRenderingContext2D, kind: ActorKind, direction: number, frame: number) {
  const angle = (direction * Math.PI) / 4,
    dx = Math.cos(angle),
    dy = Math.sin(angle);
  const back = dy < -0.35,
    side = Math.abs(dx) > 0.7;
  const stride = frame === 0 ? 0 : Math.sin(((frame - 1) / 5) * Math.PI * 2) * 5;
  const fat = kind === "velvet",
    paper = kind === "paper",
    water = kind === "watercolor",
    rat = kind === "rat";
  const body = rat
    ? "#796079"
    : paper
      ? "#3c5552"
      : water
        ? "#79798c"
        : fat
          ? "#111723"
          : "#090f1a";
  const light = rat ? "#aa8298" : paper ? "#70897a" : water ? "#b3a2a9" : "#263348";
  const cx = 48,
    cy = 58;
  const hx = cx + dx * (side ? 18 : 11),
    hy = cy + dy * 12 - (back ? 13 : 7);
  c.lineCap = "round";
  c.lineJoin = "round";
  c.strokeStyle = body;
  c.lineWidth = rat ? 4 : fat ? 12 : 8;
  c.beginPath();
  c.moveTo(cx - dx * 15, cy - dy * 9);
  c.bezierCurveTo(
    cx - dx * 35 - 8,
    cy - dy * 18,
    cx - dx * 31 - 10 + stride * 0.25,
    cy - 35,
    cx - dx * 20 - 7,
    cy - 29,
  );
  c.stroke();
  const legs = [
    [-12, -5, 1],
    [12, -5, -1],
    [-12, 8, -1],
    [12, 8, 1],
  ];
  for (const [lx = 0, ly = 0, sign = 1] of legs) {
    const footX = cx + lx * (side ? 0.8 : 1) + stride * sign * (side ? dx : 0.25);
    const footY = cy + ly + 17 + stride * sign * dy * 0.4;
    oval(c, footX, footY, rat ? 5 : 6, 5, body);
    c.strokeStyle = body;
    c.lineWidth = fat ? 10 : 7;
    c.beginPath();
    c.moveTo(cx + lx, cy + ly);
    c.lineTo(footX, footY);
    c.stroke();
  }
  if (paper) {
    shape(
      c,
      [cx - 19, cy - 14, cx + 14, cy - 19, cx + 24, cy + 12, cx + 5, cy + 22, cx - 22, cy + 13],
      body,
    );
    shape(c, [cx - 19, cy - 14, cx + 5, cy + 22, cx - 22, cy + 13], light);
  } else {
    const gradient = c.createRadialGradient(cx - 7, cy - 10, 2, cx, cy, fat ? 34 : 25);
    gradient.addColorStop(0, light);
    gradient.addColorStop(0.4, body);
    gradient.addColorStop(1, body);
    c.fillStyle = gradient;
    c.beginPath();
    c.ellipse(
      cx,
      cy,
      fat ? 29 : rat ? 22 : side ? 24 : 19,
      fat ? 24 : side ? 17 : 22,
      0,
      0,
      Math.PI * 2,
    );
    c.fill();
    if (water) {
      c.save();
      c.clip();
      for (let i = 0; i < 75; i++) {
        c.globalAlpha = 0.1;
        oval(
          c,
          22 + ((i * 17) % 52),
          30 + ((i * 23) % 48),
          3 + (i % 6),
          2 + (i % 4),
          i % 2 ? "#d7bdbb" : "#3e4a60",
        );
      }
      c.restore();
    }
  }
  if (rat) {
    oval(c, hx, hy, 14, 12, light);
    oval(c, hx - dx * 5 - 7, hy - 10, 7, 8, "#bb93a7");
    shape(c, [hx, hy - 4, hx + dx * 22, hy + dy * 8, hx, hy + 8], light);
    oval(c, hx + dx * 20, hy + dy * 8, 3, 3, "#d2a7b3");
  } else {
    if (paper)
      shape(
        c,
        [
          hx - 19,
          hy + 10,
          hx - 19,
          hy - 16,
          hx - 14,
          hy - 30,
          hx - 1,
          hy - 16,
          hx + 9,
          hy - 18,
          hx + 21,
          hy - 32,
          hx + 21,
          hy + 4,
          hx + 4,
          hy + 17,
        ],
        body,
      );
    else {
      oval(c, hx, hy, side ? 18 : 21, 19, body);
      shape(
        c,
        [
          hx - 19,
          hy - 6,
          hx - 18,
          hy - 30,
          hx - 2,
          hy - 15,
          hx + 7,
          hy - 17,
          hx + 21,
          hy - 29,
          hx + 20,
          hy + 2,
        ],
        body,
      );
      shape(c, [hx - 15, hy - 16, hx - 15, hy - 23, hx - 7, hy - 15], back ? light : "#815f7d");
      shape(c, [hx + 11, hy - 17, hx + 18, hy - 23, hx + 17, hy - 12], back ? light : "#815f7d");
    }
  }
  if (!back) {
    const fx = hx + dx * 5,
      fy = hy + (dy > 0.3 ? 5 : 0);
    const eyeColor = rat ? "#f3af9a" : "#a8f0ce";
    if (!side || dx < 0) {
      oval(c, fx - 8, fy - 1, rat ? 2 : 5, rat ? 3 : 4, eyeColor);
      oval(c, fx - 8, fy - 1, 1.2, 3.7, "#0b1720");
    }
    if (!side || dx > 0) {
      oval(c, fx + 8, fy - 1, rat ? 2 : 5, rat ? 3 : 4, rat ? eyeColor : "#bcb0ec");
      oval(c, fx + 8, fy - 1, 1.2, 3.7, "#0b1720");
    }
    if (!rat) {
      shape(c, [fx - 2, fy + 7, fx + 3, fy + 7, fx, fy + 10], "#b78a9e");
      c.strokeStyle = "#b2a6bf";
      c.lineWidth = 0.7;
      for (const sign of [-1, 1]) {
        c.beginPath();
        c.moveTo(fx + sign * 10, fy + 7);
        c.lineTo(fx + sign * 23, fy + 5);
        c.stroke();
      }
    }
  } else {
    c.strokeStyle = light;
    c.lineWidth = paper ? 1.5 : 1;
    c.globalAlpha = 0.5;
    c.beginPath();
    c.moveTo(hx - 8, hy + 3);
    c.quadraticCurveTo(hx, hy + 9, hx + 9, hy + 2);
    c.stroke();
    c.globalAlpha = 1;
  }
}

function paintFlying(
  c: CanvasRenderingContext2D,
  kind: "owl" | "unicorn",
  direction: number,
  frame: number,
) {
  const angle = (direction * Math.PI) / 4,
    dx = Math.cos(angle),
    dy = Math.sin(angle);
  const flap = Math.sin((frame / 6) * Math.PI * 2) * 9;
  if (kind === "owl") {
    shape(
      c,
      [42, 45, 6, 30 + flap, 13, 62 + flap, 43, 63, 56, 63, 85, 57 + flap, 91, 24 + flap, 57, 42],
      "#80708f",
    );
    oval(c, 49, 50, 20, 26, "#aa8aab");
    const hx = 49 + dx * 6,
      hy = 37 + dy * 5;
    shape(
      c,
      [
        hx - 18,
        hy,
        hx - 18,
        hy - 23,
        hx - 3,
        hy - 12,
        hx + 7,
        hy - 12,
        hx + 20,
        hy - 24,
        hx + 20,
        hy,
      ],
      "#aa8aab",
    );
    if (dy >= -0.4) {
      oval(c, hx - 9, hy, 10, 11, "#f5dcaf");
      oval(c, hx + 9, hy, 10, 11, "#f5dcaf");
      oval(c, hx - 8 + dx * 2, hy, 3, 6, "#2e283e");
      oval(c, hx + 8 + dx * 2, hy, 3, 6, "#2e283e");
      shape(c, [hx - 4, hy + 11, hx + 4, hy + 11, hx, hy + 19], "#e0aa80");
    }
  } else {
    const hx = 49 + dx * 20,
      hy = 44 + dy * 12;
    shape(c, [42, 50, 13, 29 + flap, 16, 59 + flap, 43, 67], "#c3b7df");
    oval(c, 45, 57, 27, 17, "#e5d8ed");
    shape(
      c,
      [
        hx - 10,
        hy + 24,
        hx - 13,
        hy,
        hx - 4,
        hy - 14,
        hx + 13,
        hy - 10,
        hx + 19,
        hy + 4,
        hx + 9,
        hy + 8,
        hx + 8,
        hy + 25,
      ],
      "#f5e7ef",
    );
    shape(c, [hx + 3, hy - 10, hx + 13 + dx * 5, hy - 32, hx + 15, hy - 6], "#ffe2a5");
    shape(c, [hx - 10, hy - 10, hx - 19, hy - 6, hx - 21, hy + 19, hx - 8, hy + 13], "#b192cf");
    for (const leg of [-1, 1]) {
      c.strokeStyle = "#e5d8ed";
      c.lineWidth = 7;
      c.lineCap = "round";
      c.beginPath();
      c.moveTo(45 + leg * 16, 65);
      c.lineTo(45 + leg * 22 + flap * 0.3, 82);
      c.stroke();
    }
    if (dy >= -0.4) oval(c, hx + 6 + dx * 5, hy - 1, 2, 3, "#7f538f");
    c.strokeStyle = "#bd8bbb";
    c.lineWidth = 7;
    c.beginPath();
    c.moveTo(23, 54);
    c.quadraticCurveTo(5, 37, 8, 67 + flap);
    c.stroke();
  }
}

export function registerActorAtlases(scene: Phaser.Scene) {
  for (const kind of actorKinds) {
    const key = `actor-${kind}`;
    if (scene.textures.exists(key)) continue;
    const sheet = scene.textures.createCanvas(key, FRAME_SIZE * WALK_FRAMES, FRAME_SIZE * 8);
    if (!sheet) throw new Error(`Could not create ${key}.`);
    for (let direction = 0; direction < 8; direction++) {
      for (let frame = 0; frame < WALK_FRAMES; frame++) {
        const c = sheet.context;
        c.save();
        c.translate(frame * FRAME_SIZE, direction * FRAME_SIZE);
        if (kind === "owl" || kind === "unicorn") paintFlying(c, kind, direction, frame);
        else paintCat(c, kind, direction, frame);
        c.restore();
        sheet.add(
          direction * WALK_FRAMES + frame,
          0,
          frame * FRAME_SIZE,
          direction * FRAME_SIZE,
          FRAME_SIZE,
          FRAME_SIZE,
        );
      }
      scene.anims.create({
        key: `${kind}-${direction}`,
        frames: Array.from({ length: WALK_FRAMES - 1 }, (_, i) => ({
          key,
          frame: direction * WALK_FRAMES + i + 1,
        })),
        frameRate: kind === "rat" ? 12 : 9,
        repeat: -1,
      });
    }
    sheet.refresh();
  }
}
