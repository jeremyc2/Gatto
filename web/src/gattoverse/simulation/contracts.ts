export type Point = { x: number; y: number };
export type Phase = "playing" | "paused" | "help" | "dialogue" | "over";
export type Mode = "roof" | "flight";
export type ActorKind = "cat" | "watercolor" | "paper" | "velvet" | "rat" | "owl" | "unicorn";
export type SpriteKind =
  | ActorKind
  | "key"
  | "food"
  | "case"
  | "coin"
  | "vendor"
  | "plant"
  | "vent"
  | "chimney"
  | "cloth"
  | "puddle"
  | "rug"
  | "crate"
  | "lantern";
export type Failure = "rat" | "owl" | "unicorn" | "hunger" | "fall" | undefined;
export type Command =
  | { type: "move"; x: number; y: number }
  | { type: "walk-to"; point: Point; entity?: number }
  | { type: "thrust"; active: boolean }
  | { type: "interact" }
  | { type: "launch" }
  | { type: "pause" | "help" | "resume" }
  | { type: "restart"; seed: number };

export type RenderEntity = Readonly<
  Point & {
    id: number;
    kind: SpriteKind;
    direction: number;
    gait: number;
    variant: number;
    scale: number;
    used: boolean;
    state: number;
  }
>;
export type LaundryLine = {
  from: Point;
  to: Point;
  sag: number;
  clothes: { t: number; variant: number; angle: number }[];
};
export type RoofLayout = Readonly<{
  id: number;
  seed: number;
  width: number;
  depth: number;
  spawn: Point;
  laundry: LaundryLine[];
  tiles: number;
}>;
export type FlightRoof = Readonly<{ id: number; x: number; y: number; width: number }>;
export type Dialogue = { name: string; kind: ActorKind; text: string };
export type GameView = Readonly<{
  seed: number;
  phase: Phase;
  mode: Mode;
  time: number;
  revision: number;
  roof: RoofLayout;
  platforms: FlightRoof[];
  player: RenderEntity;
  entities: RenderEntity[];
  stamina: number;
  keys: number;
  wallet: number;
  score: number;
  clothes: number;
  jetpack: boolean;
  thrust: boolean;
  near: { id: number; kind: "case" | "vendor" | "cat" } | undefined;
  feedback:
    | {
        icon: "key" | "coin" | "food" | "cloth" | "locked" | "pack";
        amount: number;
        serial: number;
      }
    | undefined;
  dialogue: Dialogue | undefined;
  failure: Failure;
}>;

/** The only simulation interface used by input, rendering, UI, and tests. */
export type Simulation = {
  send(command: Command): void;
  step(seconds: number): void;
  read(): GameView;
};
