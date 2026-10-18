import { addComponents, query, removeEntity } from "bitecs";
import { get } from "./components";
import type { Point, RoofLayout, SpriteKind } from "./contracts";
import { distance, random, roofSeed } from "./random";
import { addHazard, addVisitor, pointOf, spawn, type GameWorld } from "./world";

/** Rejection sampling gives independent scattered placements instead of a placement grid. */
export function populateRoof(world: GameWorld, id: number): RoofLayout {
  const cached = world.state.roofs.get(id);
  if (cached) return cached;
  const seed = roofSeed(world.seed, id);
  const rng = random(seed);
  const width = rng.integer(1800, 2280),
    depth = rng.integer(1350, 1740);
  const spawnPoint = { x: width * rng.between(0.35, 0.48), y: depth * rng.between(0.5, 0.64) };
  const occupied: { p: Point; radius: number }[] = [{ p: spawnPoint, radius: 140 }];
  const cases: Point[] = [];
  const pick = (radius: number, avoidCases = false): Point => {
    for (let attempt = 0; attempt < 2500; attempt++) {
      const p = { x: rng.between(80, width - 80), y: rng.between(80, depth - 80) };
      if (occupied.some((o) => distance(p, o.p) < o.radius + radius)) continue;
      if (avoidCases && cases.some((c) => distance(p, c) < 155)) continue;
      occupied.push({ p, radius });
      return p;
    }
    throw new Error(`Couldn't place roof contents for seed ${seed}.`);
  };
  const loot = (kind: "case" | "key" | "food" | "coin", p: Point, coins = 0) => {
    const e = spawn(world, kind, p, id, 1, rng.integer(0, 5));
    addComponents(world, e, world.components.Loot);
    world.components.Loot.used[e] = 0;
    world.components.Loot.coins[e] = coins;
    return e;
  };
  const vendor = { x: width - rng.between(130, 220), y: rng.between(140, 250) };
  occupied.push({ p: vendor, radius: 90 });
  if (id === 0) spawn(world, "vendor", vendor, id);
  for (let i = 0, n = rng.integer(24, 34); i < n; i++) {
    const p = pick(32);
    cases.push(p);
    loot("case", p, rng.next() < (id === 0 ? 0.4 : 0.17) ? rng.integer(1, 3) : 0);
  }
  for (let i = 0, n = rng.integer(8, 11); i < n; i++) loot("key", pick(20, true));
  for (let i = 0, n = rng.integer(14, 20); i < n; i++) loot("food", pick(30));
  // Loose change prevents a run becoming unwinnable when scarce keys reveal only clothes.
  for (let i = 0, n = id === 0 ? 8 : rng.integer(3, 7); i < n; i++) loot("coin", pick(18), 1);

  const furniture: { kind: SpriteKind; count: number; radius: number; blocking: boolean }[] = [
    { kind: "plant", count: rng.integer(16, 25), radius: 26, blocking: true },
    { kind: "vent", count: rng.integer(4, 8), radius: 48, blocking: true },
    { kind: "chimney", count: rng.integer(3, 6), radius: 32, blocking: true },
    { kind: "crate", count: rng.integer(8, 15), radius: 26, blocking: true },
    { kind: "lantern", count: rng.integer(9, 15), radius: 20, blocking: false },
    { kind: "rug", count: rng.integer(4, 8), radius: 35, blocking: false },
    { kind: "puddle", count: rng.integer(8, 15), radius: 32, blocking: false },
    { kind: "cloth", count: rng.integer(15, 28), radius: 14, blocking: false },
  ];
  for (const object of furniture) {
    for (let i = 0; i < object.count; i++) {
      const e = spawn(
        world,
        object.kind,
        pick(object.radius),
        id,
        rng.between(0.8, 1.25),
        rng.integer(0, 5),
      );
      if (object.blocking) {
        addComponents(world, e, world.components.Obstacle);
        world.components.Obstacle.width[e] = object.radius * 1.55;
        world.components.Obstacle.depth[e] = object.radius * 1.25;
      }
    }
  }
  for (let i = 0, n = rng.integer(2, 4); i < n; i++) {
    addVisitor(
      world,
      (["watercolor", "paper", "velvet"] as const)[(id + i) % 3] ?? "watercolor",
      pick(28),
      id,
      rng,
    );
  }
  for (let i = 0, n = id === 0 ? 1 : rng.integer(2, 4); i < n; i++) {
    let p = pick(25);
    while (distance(p, spawnPoint) < 470) p = pick(25);
    addHazard(world, "rat", p, id, rng);
  }
  const laundry: RoofLayout["laundry"] = [];
  for (let i = 0, n = rng.integer(11, 17); i < n; i++) {
    const from = { x: rng.between(110, width - 420), y: rng.between(110, depth - 350) };
    const angle = rng.between(-1, 1),
      length = rng.between(220, 440);
    const to = {
      x: Math.min(width - 80, from.x + Math.cos(angle) * length),
      y: Math.max(80, Math.min(depth - 80, from.y + Math.sin(angle) * length)),
    };
    const clothes = [];
    // Random gaps and empty stretches make each clothesline feel used, rather than filled.
    let t = rng.between(0.05, 0.22);
    while (t < 0.95) {
      clothes.push({ t, variant: rng.integer(0, 5), angle: rng.between(-11, 11) });
      t += rng.between(0.1, 0.23);
    }
    laundry.push({ from, to, sag: rng.between(4, 14), clothes });
  }
  const roof = { id, seed, width, depth, spawn: spawnPoint, laundry, tiles: rng.integer(0, 3) };
  world.state.roofs.set(id, roof);
  return roof;
}

export function ensureFlightRoofs(world: GameWorld, through: number) {
  while (world.state.nextPlatform <= through) {
    const id = world.state.nextPlatform++;
    const rng = random(roofSeed(world.seed + 7, id));
    const platform = {
      id,
      x: world.state.flightEnd,
      y: rng.between(405, 555),
      width: rng.between(410, 620),
    };
    world.state.flightEnd = platform.x + platform.width + rng.between(220, 330);
    world.state.platforms.set(id, platform);
    for (let i = 0, n = rng.integer(5, 10); i < n; i++) {
      const x = platform.x + 90 + i * rng.between(50, 75);
      const y = platform.y - 140 - Math.sin((i / n) * Math.PI) * rng.between(40, 90);
      const e = spawn(world, "coin", { x, y }, -1, 1);
      addComponents(world, e, world.components.Loot);
      world.components.Loot.used[e] = 0;
      world.components.Loot.coins[e] = 1;
    }
    if (id > 0)
      addHazard(
        world,
        "rat",
        { x: platform.x + platform.width * rng.between(0.45, 0.8), y: platform.y - 4 },
        -1,
        rng,
      );
    addHazard(
      world,
      "owl",
      { x: platform.x + platform.width + rng.between(70, 130), y: rng.between(170, 340) },
      -1,
      rng,
    );
    if (id > 1 && rng.next() < 0.48)
      addHazard(
        world,
        "unicorn",
        { x: platform.x + platform.width * 0.5, y: rng.between(150, 260) },
        -1,
        rng,
      );
  }
}

export function cleanFlight(world: GameWorld, before: number) {
  const { Resident, Position } = world.components;
  for (const e of query(world, [Resident, Position])) {
    if (e !== world.player && get(Resident.roof, e) === -1 && pointOf(world, e).x < before)
      removeEntity(world, e);
  }
  for (const [id, roof] of world.state.platforms)
    if (roof.x + roof.width < before) world.state.platforms.delete(id);
}
