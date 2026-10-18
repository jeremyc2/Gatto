import { describe, expect, test } from "bun:test";
import { createSimulation } from "../src/gattoverse/simulation";
import type { RenderEntity, Simulation } from "../src/gattoverse/simulation/contracts";

function advance(game: Simulation, seconds: number) {
  for (let i = 0; i < Math.round(seconds * 60); i++) game.step(1 / 60);
}
function nearest(game: Simulation, kind: RenderEntity["kind"]) {
  const view = game.read();
  const target = view.entities
    .filter((e) => e.kind === kind && !e.used)
    .sort(
      (a, b) =>
        Math.hypot(a.x - view.player.x, a.y - view.player.y) -
        Math.hypot(b.x - view.player.x, b.y - view.player.y),
    )[0];
  if (!target) throw new Error(`No ${kind} in this view.`);
  return target;
}
function visit(game: Simulation, target: RenderEntity) {
  game.send({ type: "walk-to", point: target, entity: target.id });
  for (let frame = 0; frame < 1800; frame++) {
    game.step(1 / 60);
    const view = game.read();
    if (view.phase === "over")
      throw new Error(`Run ended while visiting ${target.kind}: ${view.failure}`);
    if (view.phase === "dialogue" || view.near?.id === target.id) return;
    if (!view.entities.some((e) => e.id === target.id && !e.used)) return;
    if (
      target.kind === "food" &&
      Math.hypot(view.player.x - target.x, view.player.y - target.y) < 25
    )
      return;
  }
  throw new Error(`Could not reach ${target.kind}.`);
}
function earnJetpack() {
  const game = createSimulation(1);
  for (let i = 0; i < 5; i++) visit(game, nearest(game, "coin"));
  visit(game, nearest(game, "vendor"));
  game.send({ type: "interact" });
  advance(game, 0.05);
  expect(game.read().jetpack).toBe(true);
  return game;
}

describe("seeded rooftops", () => {
  test("generation is reproducible, varied, larger, and keeps keys separate from luggage", () => {
    for (let seed = 0; seed < 30; seed++) {
      const view = createSimulation(seed).read();
      expect(view.roof.width).toBeGreaterThanOrEqual(1800);
      expect(view.roof.depth).toBeGreaterThanOrEqual(1350);
      const keys = view.entities.filter((e) => e.kind === "key"),
        cases = view.entities.filter((e) => e.kind === "case");
      expect(cases.length).toBeGreaterThan(keys.length * 2);
      for (const key of keys)
        for (const luggage of cases)
          expect(Math.hypot(key.x - luggage.x, key.y - luggage.y)).toBeGreaterThanOrEqual(155);
      expect(view.entities.filter((e) => e.kind === "coin").length).toBeGreaterThanOrEqual(5);
      expect(view.entities.length).toBeGreaterThan(120);
      expect(view.roof.laundry.length).toBeGreaterThan(10);
      expect(view.entities.map(({ kind, x, y }) => ({ kind, x, y }))).toEqual(
        createSimulation(seed)
          .read()
          .entities.map(({ kind, x, y }) => ({ kind, x, y })),
      );
    }
    expect(createSimulation(1).read().roof).not.toEqual(createSimulation(2).read().roof);
  });

  test("world stores are isolated and restarting makes a fresh run", () => {
    const first = createSimulation(44),
      second = createSimulation(44);
    first.send({ type: "move", x: 1, y: 0 });
    advance(first, 0.3);
    expect(first.read().player.x).not.toBe(second.read().player.x);
    first.send({ type: "restart", seed: 44 });
    expect(first.read()).toEqual(second.read());
  });
});

describe("simulation commands", () => {
  test("fixed steps produce the same result at different rendering rates", () => {
    const a = createSimulation(123),
      b = createSimulation(123);
    a.send({ type: "move", x: 1, y: 0 });
    b.send({ type: "move", x: 1, y: 0 });
    advance(a, 1);
    for (let i = 0; i < 30; i++) b.step(1 / 30);
    expect(a.read()).toEqual(b.read());
  });

  test("the player faces all eight movement directions and uses walking frames", () => {
    const vectors = [
      [1, 0],
      [1, 1],
      [0, 1],
      [-1, 1],
      [-1, 0],
      [-1, -1],
      [0, -1],
      [1, -1],
    ];
    for (const [direction, [x = 0, y = 0]] of vectors.entries()) {
      const game = createSimulation(123);
      game.send({ type: "move", x, y });
      advance(game, 0.2);
      expect(game.read().player.direction).toBe(direction);
      expect(game.read().player.gait).toBeGreaterThan(0);
    }
  });

  test("pause and help freeze movement, AI and stamina, and discard held movement", () => {
    const game = createSimulation(2);
    game.send({ type: "move", x: -1, y: 1 });
    advance(game, 0.2);
    game.send({ type: "pause" });
    const paused = game.read();
    game.send({ type: "move", x: 1, y: 1 });
    advance(game, 10);
    expect(game.read()).toEqual(paused);
    game.send({ type: "resume" });
    const before = game.read().player;
    advance(game, 0.3);
    expect(game.read().player.x).toBe(before.x);
    expect(game.read().player.y).toBe(before.y);
    game.send({ type: "help" });
    const helped = game.read();
    advance(game, 3);
    expect(game.read()).toEqual(helped);
  });

  test("NPCs wander on their own and conversations pause the world", () => {
    const game = createSimulation(1);
    const before = nearest(game, "watercolor");
    advance(game, 5);
    const after = game.read().entities.find((e) => e.id === before.id);
    expect(after?.x === before.x && after?.y === before.y).toBe(false);
    visit(game, nearest(game, "watercolor"));
    game.send({ type: "interact" });
    advance(game, 0.05);
    expect(game.read().phase).toBe("dialogue");
    expect(game.read().dialogue?.text.length).toBeGreaterThan(50);
    const paused = game.read();
    advance(game, 5);
    expect(game.read()).toEqual(paused);
    game.send({ type: "resume" });
    expect(game.read().phase).toBe("playing");
  });

  test("a key is collected only once and a locked suitcase cannot grant rewards", () => {
    const game = createSimulation(1);
    visit(game, nearest(game, "case"));
    game.send({ type: "interact" });
    advance(game, 0.05);
    expect(game.read().feedback?.icon).toBe("locked");
    const key = nearest(game, "key");
    visit(game, key);
    expect(game.read().keys).toBe(1);
    game.send({ type: "walk-to", point: key, entity: key.id });
    advance(game, 0.5);
    expect(game.read().keys).toBe(1);
    const target = nearest(game, "case");
    visit(game, target);
    game.send({ type: "interact" });
    advance(game, 0.05);
    const opened = game.read();
    expect(opened.keys).toBe(0);
    expect(opened.entities.find((e) => e.id === target.id)?.used).toBe(true);
    game.send({ type: "interact" });
    advance(game, 0.05);
    expect(game.read().score).toBe(opened.score);
  });

  test("touching a pursuing rat ends the run", () => {
    const game = createSimulation(2);
    const rat = nearest(game, "rat");
    game.send({ type: "walk-to", point: rat, entity: rat.id });
    for (let i = 0; i < 900 && game.read().phase === "playing"; i++) game.step(1 / 60);
    expect(game.read().phase).toBe("over");
    expect(game.read().failure).toBe("rat");
    const ended = game.read();
    advance(game, 3);
    expect(game.read()).toEqual(ended);
  });
});

test("coin economy, takeoff, landing, refueling and re-equipping work without a renderer", () => {
  const game = earnJetpack();
  expect(game.read().score).toBe(5);
  expect(game.read().wallet).toBe(0);
  game.send({ type: "launch" });
  expect(game.read().mode).toBe("flight");
  for (let i = 0; i < 210; i++) {
    game.send({ type: "thrust", active: i < 50 });
    game.step(1 / 60);
  }
  expect(game.read().mode).toBe("roof");
  expect(game.read().roof.id).toBe(1);
  expect(game.read().jetpack).toBe(true);
  const stamina = game.read().stamina;
  visit(game, nearest(game, "food"));
  expect(game.read().stamina).toBeGreaterThan(stamina);
  game.send({ type: "pause" });
  game.send({ type: "launch" });
  expect(game.read().phase).toBe("playing");
  expect(game.read().mode).toBe("flight");
});

test("the simulation imports no renderer, browser, storage, or Phaser code", async () => {
  const directory = new URL("../src/gattoverse/simulation/", import.meta.url);
  for await (const path of new Bun.Glob("**/*.ts").scan(directory.pathname)) {
    const source = await Bun.file(new URL(path, directory)).text();
    expect(source).not.toMatch(/from\s+["'][^"']*(?:phaser|rendering|\/ui\/|\/input\/)[^"']*["']/);
    expect(source).not.toMatch(/\b(?:window|document|localStorage|requestAnimationFrame)\s*[.(]\w/);
  }
});
