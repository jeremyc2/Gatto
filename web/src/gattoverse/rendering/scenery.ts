import Phaser from "phaser";
import type { FlightRoof, RoofLayout } from "../simulation/contracts";
import { random } from "../simulation/random";
import { project } from "./projection";

export type Scenery = {
  objects: Phaser.GameObjects.GameObject[];
  clothes: { image: Phaser.GameObjects.Image; ground: number }[];
};

function poly(
  g: Phaser.GameObjects.Graphics,
  points: { x: number; y: number }[],
  color: number,
  alpha = 1,
) {
  g.fillStyle(color, alpha).fillPoints(
    points.map((p) => new Phaser.Math.Vector2(p.x, p.y)),
    true,
  );
}

export function paintRoof(scene: Phaser.Scene, roof: RoofLayout, reduced: boolean): Scenery {
  const rng = random(roof.seed + 11),
    objects: Scenery["objects"] = [],
    clothes: Scenery["clothes"] = [];
  const a = project({ x: 0, y: 0 }),
    b = project({ x: roof.width, y: 0 });
  const c = project({ x: roof.width, y: roof.depth }),
    d = project({ x: 0, y: roof.depth });
  const g = scene.add.graphics().setDepth(-10000);
  objects.push(g);
  poly(g, [b, c, { x: c.x, y: c.y + 800 }, { x: b.x, y: b.y + 800 }], 0x282b40);
  poly(g, [d, c, { x: c.x, y: c.y + 800 }, { x: d.x, y: d.y + 800 }], 0x393246);
  // Architectural windows have uneven widths, lit rooms, and dark shutters.
  for (const [from, to] of [
    [d, c],
    [b, c],
  ]) {
    if (!from || !to) continue;
    for (let t = 0.03; t < 0.98; t += rng.between(0.025, 0.045)) {
      const x = from.x + (to.x - from.x) * t,
        y = from.y + (to.y - from.y) * t;
      const slope = Math.sign(to.y - from.y) * Math.sign(to.x - from.x);
      for (let row = 0; row < 8; row++) {
        const w = rng.between(20, 32),
          top = y + 45 + row * 78;
        poly(
          g,
          [
            { x, y: top },
            { x: x + w, y: top + w * 0.49 * slope },
            { x: x + w, y: top + 42 + w * 0.49 * slope },
            { x, y: top + 42 },
          ],
          rng.next() < 0.52 ? 0xcbb08e : 0x171d2e,
          rng.between(0.55, 0.95),
        );
        g.lineStyle(2, 0x776174).lineBetween(
          x + w / 2,
          top + w * 0.24 * slope,
          x + w / 2,
          top + 42 + w * 0.24 * slope,
        );
      }
    }
  }
  poly(g, [a, b, c, d], [0x676174, 0x6b6376, 0x626371, 0x6b626e][roof.tiles] ?? 0x676174);
  for (let i = 0; i < 2600; i++) {
    const p = project({ x: rng.between(10, roof.width - 10), y: rng.between(10, roof.depth - 10) });
    g.fillStyle(i % 3 === 0 ? 0xc0a8ae : 0x2b3047, rng.between(0.04, 0.12)).fillEllipse(
      p.x,
      p.y,
      rng.between(1, 5),
      rng.between(1, 2),
    );
  }
  for (let i = 0; i < 35; i++) {
    const p = project({ x: rng.between(80, roof.width - 80), y: rng.between(80, roof.depth - 80) });
    g.lineStyle(1, 0x403c53, 0.4).lineBetween(
      p.x,
      p.y,
      p.x + rng.between(-45, 45),
      p.y + rng.between(-18, 18),
    );
  }
  const edge = scene.add.graphics().setDepth(-9000);
  objects.push(edge);
  for (const [from, to] of [
    [a, b],
    [a, d],
    [d, c],
    [b, c],
  ]) {
    if (!from || !to) continue;
    poly(edge, [from, to, { x: to.x, y: to.y - 13 }, { x: from.x, y: from.y - 13 }], 0x95899b);
    edge.lineStyle(3, 0xc0aeb7, 0.8).lineBetween(from.x, from.y - 13, to.x, to.y - 13);
  }
  for (const line of roof.laundry) {
    const from = project(line.from),
      to = project(line.to);
    const wire = scene.add.graphics().setDepth(Math.min(from.y, to.y) - 1);
    objects.push(wire);
    wire.lineStyle(2, 0xc4aeb5, 0.75);
    const curve = new Phaser.Curves.QuadraticBezier(
      new Phaser.Math.Vector2(from.x, from.y - 98),
      new Phaser.Math.Vector2((from.x + to.x) / 2, (from.y + to.y) / 2 - 98 + line.sag * 2),
      new Phaser.Math.Vector2(to.x, to.y - 98),
    );
    wire.strokePoints(curve.getPoints(20));
    for (const p of [from, to]) {
      const post = scene.add.graphics().setDepth(p.y);
      objects.push(post);
      post.lineStyle(4, 0x9d8c9e).lineBetween(p.x, p.y, p.x, p.y - 105);
      post.lineStyle(1, 0xd0bdbf).lineBetween(p.x - 1, p.y, p.x - 1, p.y - 105);
    }
    for (const cloth of line.clothes) {
      const x = from.x + (to.x - from.x) * cloth.t,
        y = from.y + (to.y - from.y) * cloth.t;
      const sag = Math.sin(cloth.t * Math.PI) * line.sag;
      const image = scene.add
        .image(x, y - 96 + sag, `cloth-${cloth.variant}`)
        .setScale(rng.between(0.62, 0.86))
        .setAngle(cloth.angle)
        .setOrigin(0.5, 0)
        .setDepth(y + 1);
      objects.push(image);
      clothes.push({ image, ground: y });
      if (!reduced)
        scene.tweens.add({
          targets: image,
          angle: cloth.angle + rng.between(4, 11),
          duration: rng.between(2100, 4300),
          yoyo: true,
          repeat: -1,
          ease: "Sine.InOut",
        });
    }
  }
  return { objects, clothes };
}

export function paintFlightRoof(scene: Phaser.Scene, roof: FlightRoof, seed: number) {
  const rng = random(seed ^ roof.id),
    g = scene.add.graphics().setDepth(-20);
  g.fillStyle(roof.id % 2 ? 0x3c334b : 0x2e3046).fillRect(roof.x, roof.y, roof.width, 900);
  g.fillStyle(0x9a8a9e).fillRect(roof.x - 4, roof.y, roof.width + 8, 9);
  g.fillStyle(0xc0aeb7).fillRect(roof.x - 4, roof.y, roof.width + 8, 2);
  for (let x = 20; x < roof.width - 30; x += rng.between(45, 65)) {
    for (let y = 32; y < 800; y += 68) {
      const width = rng.between(21, 32);
      g.fillStyle(rng.next() < 0.6 ? 0xcdb28f : 0x171e31, 0.9).fillRoundedRect(
        roof.x + x,
        roof.y + y,
        width,
        38,
        2,
      );
      g.lineStyle(2, 0x7c667b).lineBetween(
        roof.x + x + width / 2,
        roof.y + y,
        roof.x + x + width / 2,
        roof.y + y + 38,
      );
    }
  }
  const objects: Phaser.GameObjects.GameObject[] = [g];
  for (let i = 0; i < 4; i++)
    objects.push(
      scene.add
        .image(roof.x + 50 + i * 60, roof.y - 36, `cloth-${rng.integer(0, 5)}`)
        .setScale(0.4)
        .setDepth(-10),
    );
  g.lineStyle(2, 0xa391a6).lineBetween(roof.x + 23, roof.y - 55, roof.x + 270, roof.y - 55);
  g.lineStyle(3, 0x97859a)
    .lineBetween(roof.x + 23, roof.y, roof.x + 23, roof.y - 60)
    .lineBetween(roof.x + 270, roof.y, roof.x + 270, roof.y - 60);
  return objects;
}
