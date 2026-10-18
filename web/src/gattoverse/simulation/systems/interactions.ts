import { hasComponent, query } from "bitecs";
import { get } from "../components";
import type { GameView } from "../contracts";
import { catStories } from "../lore";
import { distance } from "../random";
import { feedback, isCurrent, kindOf, pointOf, spawn, stopInput, type GameWorld } from "../world";

export function interactions(world: GameWorld) {
  const { Loot, Inventory, Position, Visitor } = world.components;
  const p = world.player,
    here = pointOf(world, p);
  world.state.near = undefined;
  let requested: GameView["near"];
  let nearest = Infinity;
  for (const e of query(world, [Position])) {
    if (e === p || !isCurrent(world, e)) continue;
    const kind = kindOf(world, e),
      range = distance(here, pointOf(world, e));
    const used = hasComponent(world, e, Loot) && get(Loot.used, e) === 1;
    if (used) continue;
    if (hasComponent(world, e, Loot) && range < (kind === "coin" ? 32 : 35)) {
      if (kind === "key") {
        Inventory.keys[p] = get(Inventory.keys, p) + 1;
        Loot.used[e] = 1;
        feedback(world, "key");
      } else if (kind === "food" && get(Inventory.stamina, p) < 87) {
        Inventory.stamina[p] = Math.min(100, get(Inventory.stamina, p) + 42);
        Loot.used[e] = 1;
        feedback(world, "food", 42);
      } else if (kind === "coin") {
        const amount = get(Loot.coins, e) || 1;
        Inventory.wallet[p] = get(Inventory.wallet, p) + amount;
        Inventory.score[p] = get(Inventory.score, p) + amount;
        Loot.used[e] = 1;
        feedback(world, "coin", amount);
      }
    }
    if (world.state.mode !== "roof") continue;
    const type =
      kind === "case"
        ? "case"
        : kind === "vendor"
          ? "vendor"
          : hasComponent(world, e, Visitor)
            ? "cat"
            : undefined;
    if (type && range < (type === "vendor" ? 100 : 80) && range < nearest) {
      nearest = range;
      world.state.near = { id: e, kind: type };
    }
    if (type && e === world.state.interactTarget && range < (type === "vendor" ? 100 : 80))
      requested = { id: e, kind: type };
  }
  if (!world.state.interact) return;
  world.state.interact = false;
  const near = world.state.interactTarget === undefined ? world.state.near : requested;
  world.state.interactTarget = undefined;
  if (!near) return;
  if (near.kind === "case") {
    if (!get(Inventory.keys, p)) {
      feedback(world, "locked");
      return;
    }
    Inventory.keys[p] = get(Inventory.keys, p) - 1;
    Loot.used[near.id] = 1;
    const amount = get(Loot.coins, near.id);
    if (amount) {
      Inventory.wallet[p] = get(Inventory.wallet, p) + amount;
      Inventory.score[p] = get(Inventory.score, p) + amount;
      feedback(world, "coin", amount);
    } else {
      Inventory.clothes[p] = get(Inventory.clothes, p) + 1;
      feedback(world, "cloth");
      const at = pointOf(world, near.id);
      spawn(
        world,
        "cloth",
        { x: at.x + 22, y: at.y + 15 },
        world.state.roofId,
        0.8,
        world.rng.integer(0, 5),
      );
    }
  } else if (near.kind === "vendor") {
    if (get(Inventory.jetpack, p)) return;
    if (get(Inventory.wallet, p) < 5) {
      feedback(world, "locked");
      return;
    }
    Inventory.wallet[p] = get(Inventory.wallet, p) - 5;
    Inventory.jetpack[p] = 1;
    feedback(world, "pack");
  } else {
    const kind = kindOf(world, near.id);
    if (kind !== "watercolor" && kind !== "paper" && kind !== "velvet") return;
    const lore = catStories[kind],
      count = get(Visitor.conversations, near.id);
    world.state.dialogue = {
      name: lore.name,
      kind,
      text: lore.stories[count % lore.stories.length] ?? "I'm trying to get home.",
    };
    Visitor.conversations[near.id] = count + 1;
    world.state.phase = "dialogue";
    stopInput(world);
  }
}
