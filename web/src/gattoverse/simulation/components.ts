/** Stores are allocated per world. No simulation data is shared between runs. */
export function components() {
  return {
    Position: {
      x: new Float64Array(32768),
      y: new Float64Array(32768),
      previousX: new Float64Array(32768),
      previousY: new Float64Array(32768),
    },
    Velocity: { x: new Float64Array(32768), y: new Float64Array(32768) },
    Visual: {
      kind: new Uint8Array(32768),
      direction: new Uint8Array(32768),
      gait: new Float32Array(32768),
      variant: new Uint8Array(32768),
      scale: new Float32Array(32768),
    },
    Resident: { roof: new Int32Array(32768) },
    Collider: { radius: new Float32Array(32768) },
    Obstacle: { width: new Float32Array(32768), depth: new Float32Array(32768) },
    Loot: { used: new Uint8Array(32768), coins: new Uint8Array(32768) },
    Brain: {
      state: new Uint8Array(32768),
      timer: new Float32Array(32768),
      targetX: new Float64Array(32768),
      targetY: new Float64Array(32768),
      homeX: new Float64Array(32768),
      homeY: new Float64Array(32768),
      speed: new Float32Array(32768),
    },
    Inventory: {
      stamina: new Float32Array(32768),
      keys: new Uint16Array(32768),
      wallet: new Uint32Array(32768),
      score: new Uint32Array(32768),
      clothes: new Uint32Array(32768),
      jetpack: new Uint8Array(32768),
    },
    Player: {},
    Hazard: {},
    Visitor: { conversations: new Uint8Array(32768) },
  };
}

export const spriteKinds = [
  "cat",
  "watercolor",
  "paper",
  "velvet",
  "rat",
  "owl",
  "unicorn",
  "key",
  "food",
  "case",
  "coin",
  "vendor",
  "plant",
  "vent",
  "chimney",
  "cloth",
  "puddle",
  "rug",
  "crate",
  "lantern",
] as const;
export const kindIndex = (kind: (typeof spriteKinds)[number]) => spriteKinds.indexOf(kind);
export const get = (store: ArrayLike<number>, id: number) => store[id] ?? 0;
