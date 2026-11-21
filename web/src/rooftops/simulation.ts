import { addAchievement, recordScoreAchievements } from "./achievements";
import {
  ARRIVAL_FLAME_OUT,
  clamp,
  DESTINATION_INSET,
  GRAVITY,
  JUMP,
  LEVELS,
  REST_ROOF_WIDTH,
  SPEED,
  START_X,
  STEP,
  WORLD_UNITS_PER_METER,
  type Coin,
  type Enemy,
  type Frame,
  type GhostGap,
  type Input,
  type Roof,
  type Score,
} from "./types";

export const ROOFS: readonly Roof[] = [
  { x: -100, y: 430, width: 430 },
  { x: 390, y: 390, width: 170 },
  { x: 635, y: 340, width: 145 },
  { x: 875, y: 385, width: 180 },
  { x: 1140, y: 320, width: 200 },
  { x: 1420, y: 360, width: 140 },
  { x: 1640, y: 295, width: 200 },
  { x: 1930, y: 350, width: 180 },
  { x: 2190, y: 300, width: 240 },
  { x: 2530, y: 365, width: 170 },
  { x: 2790, y: 310, width: 170 },
  { x: 3050, y: 350, width: 240 },
  { x: 3370, y: 285, width: 210 },
  { x: 3670, y: 320, width: 200 },
  { x: 3950, y: 260, width: 200 },
  { x: 4230, y: 315, width: 200 },
  { x: 4510, y: 360, width: REST_ROOF_WIDTH },
];

const LEVEL_HEIGHTS: readonly number[] = [
  430, 380, 425, 350, 400, 340, 410, 335, 390, 320, 375, 305, 370, 320, 380, 310, 360, 300, 360,
  310, 360,
];

const GAPS_PATTERN = [115, 135, 125] as const;
const LEVEL1_WIDTHS = [150, 135, 170, 125] as const;
const LEVEL2_WIDTHS = [115, 145, 125] as const;

export function levelRoofs(level: number): Roof[] {
  if (level === 0) return ROOFS.map((r) => ({ ...r }));
  const count = ROOFS.length + level * 2;
  const roofs: Roof[] = [
    {
      // The previous destination, translated so its doorway is at START_X.
      x: START_X - (REST_ROOF_WIDTH - DESTINATION_INSET),
      y: 360,
      width: REST_ROOF_WIDTH,
    },
  ];
  for (let i = 1; i < count; i++) {
    const previous = roofs[i - 1]!;
    const y = i === count - 1 ? 360 : (LEVEL_HEIGHTS[i] ?? 360);
    const gap = previous.y - y > 60 ? 105 : (GAPS_PATTERN[i % 3] ?? 125) + level * 3;
    const width =
      i === count - 1
        ? REST_ROOF_WIDTH
        : level === 1
          ? (LEVEL1_WIDTHS[i % 4] ?? 150)
          : (LEVEL2_WIDTHS[i % 3] ?? 125);
    roofs.push({
      x: previous.x + previous.width + gap,
      y,
      width,
    });
  }
  return roofs;
}

const courseOffsets: readonly number[] = (() => {
  const offsets: number[] = [0];
  for (let level = 0; level < LEVELS; level++) {
    const roofs = levelRoofs(level);
    const last = roofs[roofs.length - 1]!;
    offsets.push(offsets[level]! + last.x + last.width - DESTINATION_INSET - START_X);
  }
  return offsets;
})();

export function courseProgress(level: number, x: number): number {
  const base = courseOffsets[level] ?? 0;
  const next = courseOffsets[level + 1] ?? base;
  return base + clamp(x - START_X, 0, next - base);
}

export function ghostInView(ghost: Frame, level: number, camera: number, width: number): boolean {
  if (ghost.level !== level) return false;
  const [left, right] = ghost.pose.home ? [-11, 29] : ghost.pose.facing > 0 ? [-27, 20] : [-20, 32];
  return ghost.pose.x + right >= camera && ghost.pose.x + left <= camera + width;
}

export function ghostGap(
  player: Run,
  ghost: Frame,
  camera: number,
  width: number,
): GhostGap | null {
  if (ghostInView(ghost, player.level, camera, width)) return null;
  const delta = courseProgress(player.level, player.x) - courseProgress(ghost.level, ghost.pose.x);
  return {
    ahead: ghost.level !== player.level ? player.level > ghost.level : delta > 0,
    meters: Math.abs(delta) / WORLD_UNITS_PER_METER,
  };
}

export function enemyPosition(
  enemy: Enemy,
  roofs: readonly Roof[],
  time: number,
): [number, number] {
  const roof = roofs[enemy.roof]!;
  const swing = Math.sin(time * enemy.speed + enemy.phase);
  if (enemy.kind === "rat") {
    return [roof.x + roof.width * 0.5 + swing * (roof.width * 0.5 - 28), roof.y - 11];
  }
  return [
    roof.x + roof.width * 0.55 + swing * 70,
    roof.y - 78 + Math.sin(time * 2.1 + enemy.phase) * 20,
  ];
}

export function arrivalFlamesOn(time: number): boolean {
  return time < ARRIVAL_FLAME_OUT && (time < 0.45 || Math.floor(time * 18) % 3 !== 0);
}

type JumpChain = "none" | "jumped" | "bouncedOffFlyer";

class Stunts {
  jump: JumpChain = "none";
  bouncedFrom: number | null = null;
  unlocked = 0;

  clone(): Stunts {
    const copy = new Stunts();
    copy.jump = this.jump;
    copy.bouncedFrom = this.bouncedFrom;
    copy.unlocked = this.unlocked;
    return copy;
  }

  stomp(enemy: Enemy): void {
    if (this.jump === "jumped" && (enemy.kind === "owl" || enemy.kind === "bat")) {
      this.jump = "bouncedOffFlyer";
    } else if (this.jump === "bouncedOffFlyer" && enemy.kind === "rat") {
      this.unlocked = addAchievement(this.unlocked, "DoubleFeature");
    }
    this.bouncedFrom = enemy.roof;
  }

  land(roof: number): void {
    if (this.bouncedFrom !== null && this.bouncedFrom + 1 === roof) {
      this.unlocked = addAchievement(this.unlocked, "BorrowedLift");
    }
    this.clearFlight();
  }

  clearFlight(): void {
    this.jump = "none";
    this.bouncedFrom = null;
  }
}

export class Run {
  level: number;
  bankedCoins = 0;
  bankedReturns = 0;
  bankedKills = 0;
  elapsed = 0;
  roofs: Roof[];
  coins: Coin[];
  enemies: Enemy[];
  stunts = new Stunts();
  x = START_X;
  y: number;
  vx = 0;
  vy = 0;
  facing = 1;
  ground: number | null = 0;
  checkpoint = 0;
  coyote = 0.12;
  buffer = 0;
  squash = 0;
  time = 0;
  jumps = 0;
  returns = 0;
  lit: boolean[];
  invulnerable = 0;
  input: Input = { left: false, right: false, jump: false };
  home = false;

  private constructor(level: number) {
    this.level = level;
    this.roofs = levelRoofs(level);
    this.y = this.roofs[0]!.y;
    this.lit = this.roofs.map((_, i) => i === 0);
    this.coins = this.roofs.map((roof, i) => {
      if (i > 0 && i % 2 === 1) {
        const previous = this.roofs[i - 1]!;
        return {
          x: (previous.x + previous.width + roof.x) * 0.5,
          y: (previous.y + roof.y) * 0.5 - 78,
          collected: false,
        };
      }
      return {
        x: roof.x + roof.width * 0.72,
        y: roof.y - 104,
        collected: false,
      };
    });
    this.enemies = [];
    for (let i = 2; i < this.roofs.length - 1; i++) {
      if (i % (level === 2 ? 2 : 3) === 0) {
        this.enemies.push({
          roof: i,
          kind: "rat",
          defeated: false,
          phase: i * 1.7,
          speed: 1 + level * 0.35,
        });
      }
      if (level > 0 && i % 5 === 2) {
        this.enemies.push({
          roof: i,
          kind: level === 1 ? "bat" : "owl",
          defeated: false,
          phase: i,
          speed: 0.8 + level * 0.25,
        });
      }
    }
  }

  static forLevel(level: number): Run {
    return new Run(level);
  }

  static forArrival(): Run {
    const run = new Run(0);
    run.lit[0] = false;
    return run;
  }

  clone(): Run {
    const copy = new Run(this.level);
    copy.bankedCoins = this.bankedCoins;
    copy.bankedReturns = this.bankedReturns;
    copy.bankedKills = this.bankedKills;
    copy.elapsed = this.elapsed;
    copy.roofs = this.roofs.map((r) => ({ ...r }));
    copy.coins = this.coins.map((c) => ({ ...c }));
    copy.enemies = this.enemies.map((e) => ({ ...e }));
    copy.stunts = this.stunts.clone();
    copy.x = this.x;
    copy.y = this.y;
    copy.vx = this.vx;
    copy.vy = this.vy;
    copy.facing = this.facing;
    copy.ground = this.ground;
    copy.checkpoint = this.checkpoint;
    copy.coyote = this.coyote;
    copy.buffer = this.buffer;
    copy.squash = this.squash;
    copy.time = this.time;
    copy.jumps = this.jumps;
    copy.returns = this.returns;
    copy.lit = [...this.lit];
    copy.invulnerable = this.invulnerable;
    copy.input = { ...this.input };
    copy.home = this.home;
    return copy;
  }

  jumpDown(): void {
    if (this.home) {
      const next = (this.level + 1) % LEVELS;
      const score = this.score();
      const time = this.time;
      const lastCoin = this.coins[this.coins.length - 1];
      const doorwayCoinCollected = lastCoin ? lastCoin.collected : false;
      const stuntsUnlocked = this.stunts.unlocked;
      const fresh = Run.forLevel(next);
      Object.assign(this, fresh);
      if (next !== 0) {
        this.bankedCoins = score.coins;
        this.bankedReturns = score.deaths;
        this.bankedKills = score.kills;
        this.elapsed = score.seconds;
        this.time = time;
        this.stunts.unlocked = stuntsUnlocked;
        if (doorwayCoinCollected) {
          this.coins.shift();
        }
      }
    } else if (!this.input.jump) {
      this.input.jump = true;
      this.buffer = 0.12;
    }
  }

  jumpUp(): void {
    this.input.jump = false;
    // Releasing early gives small hops; holding gives the full arc.
    if (this.vy < -200) {
      this.vy *= 0.55;
    }
  }

  step(): void {
    this.time += STEP;
    this.invulnerable = Math.max(0, this.invulnerable - STEP);
    this.squash = Math.max(0, this.squash - STEP * 5);
    if (this.home) return;

    this.buffer = Math.max(0, this.buffer - STEP);
    this.coyote = this.ground !== null ? 0.12 : Math.max(0, this.coyote - STEP);
    const axis = (this.input.right ? 1 : 0) - (this.input.left ? 1 : 0);
    const target = axis * SPEED;
    const acceleration = axis === 0 ? 2200 : 1850;
    this.vx += clamp(target - this.vx, -acceleration * STEP, acceleration * STEP);
    if (axis !== 0) {
      this.facing = axis;
    }
    if (this.buffer > 0 && this.coyote > 0) {
      this.vy = -JUMP;
      this.ground = null;
      this.buffer = 0;
      this.coyote = 0;
      this.jumps += 1;
      this.stunts.jump = "jumped";
      this.stunts.bouncedFrom = null;
    }

    const previousY = this.y;
    this.moveHorizontal();
    this.vy += GRAVITY * STEP;
    this.y += this.vy * STEP;
    const previousGround = this.ground;
    this.ground = null;

    if (this.invulnerable === 0) {
      for (const enemy of this.enemies) {
        if (enemy.defeated) continue;
        const [ex, ey] = enemyPosition(enemy, this.roofs, this.time);
        const top = ey - 11;
        if (Math.abs(this.x - ex) < 25 && this.y > top && this.y - 30 < ey + 11) {
          if (this.vy > 0 && previousY <= top + 5) {
            enemy.defeated = true;
            this.stunts.stomp(enemy);
            this.y = top;
            this.vy = this.input.jump ? -JUMP : -410;
            this.coyote = 0;
            break;
          } else {
            this.respawn();
            return;
          }
        }
      }
    }

    if (this.vy >= 0) {
      for (let i = 0; i < this.roofs.length; i++) {
        const roof = this.roofs[i]!;
        if (
          this.x + 11 > roof.x &&
          this.x - 11 < roof.x + roof.width &&
          previousY <= roof.y + 0.1 &&
          this.y >= roof.y
        ) {
          this.y = roof.y;
          this.vy = 0;
          this.ground = i;
          this.stunts.land(i);
          if (previousGround !== i) {
            this.squash = 1;
            this.lit[i] = true;
            if (i % this.checkpointSpacing() === 0) {
              this.checkpoint = i;
            }
          }
          break;
        }
      }
    }

    for (const coin of this.coins) {
      if (
        !coin.collected &&
        Math.abs(this.x - coin.x) < 19 &&
        Math.abs(this.y - 20 - coin.y) < 22
      ) {
        coin.collected = true;
      }
    }

    if (this.y > 720) {
      this.respawn();
    }

    if (this.x >= this.destination() && this.ground === this.roofs.length - 1) {
      this.x = this.destination();
      this.home = true;
      this.vx = 0;
      this.vy = 0;
      this.squash = 0;
      this.facing = 1;
      this.buffer = 0;
      this.input = { left: false, right: false, jump: false };
    }
  }

  score(): Score {
    let kills = this.bankedKills;
    for (const enemy of this.enemies) {
      if (enemy.defeated) kills += 1;
    }
    return {
      coins: this.coinCount(),
      deaths: this.bankedReturns + this.returns,
      kills,
      seconds: this.elapsed,
      personalBest: null,
    };
  }

  recordActiveTime(elapsed: number): void {
    if (!this.home) {
      this.elapsed += elapsed;
    }
  }

  completionAchievements(score: Score): number {
    let unlocked = recordScoreAchievements(this.stunts.unlocked, score);
    if (score.coins === COURSE_TOTALS[0] && score.kills === COURSE_TOTALS[1]) {
      unlocked = addAchievement(unlocked, "CleanSweep");
    }
    return unlocked;
  }

  ghostFrame(): Frame {
    return {
      seconds: this.elapsed,
      level: this.level,
      returns: this.bankedReturns + this.returns,
      pose: {
        x: this.x,
        y: this.y,
        facing: this.facing,
        squash: this.squash,
        home: this.home,
        gait:
          this.ground === null ? 1.5 : Math.sin(this.time * 26) * (Math.abs(this.vx) / SPEED) * 2.5,
      },
    };
  }

  checkpointSpacing(): number {
    return this.level === 0 ? 4 : 5;
  }

  roofIdentity(index: number): [number, number] {
    if (index === 0 && this.level > 0) {
      return [this.level - 1, ROOFS.length + (this.level - 1) * 2 - 1];
    }
    return [this.level, index];
  }

  hasLantern(index: number): boolean {
    return index > 0 && index + 1 < this.roofs.length && index % this.checkpointSpacing() === 0;
  }

  coinCount(): number {
    let count = this.bankedCoins;
    for (const coin of this.coins) {
      if (coin.collected) count += 1;
    }
    return count;
  }

  litCount(): number {
    let count = 0;
    for (const isLit of this.lit) {
      if (isLit) count += 1;
    }
    return count;
  }

  destination(): number {
    const last = this.roofs[this.roofs.length - 1]!;
    return last.x + last.width - DESTINATION_INSET;
  }

  respawn(): void {
    this.stunts.clearFlight();
    const roof = this.roofs[this.checkpoint]!;
    this.x = this.checkpoint === 0 ? START_X : roof.x + 24;
    this.y = roof.y;
    this.vx = 0;
    this.vy = 0;
    this.ground = this.checkpoint;
    this.coyote = 0.12;
    this.buffer = 0;
    this.invulnerable = 1.2;
    this.returns += 1;
  }

  private moveHorizontal(): void {
    const previousX = this.x;
    this.x = clamp(this.x + this.vx * STEP, 15, this.destination() + 35);
    let blocked = false;
    for (const roof of this.roofs) {
      if (this.y <= roof.y + 0.1) continue;
      const right = roof.x + roof.width;
      if (previousX + 11 <= roof.x + 0.1 && this.x + 11 > roof.x) {
        this.x = Math.min(this.x, roof.x - 11);
        blocked = true;
      } else if (previousX - 11 >= right - 0.1 && this.x - 11 < right) {
        this.x = Math.max(this.x, right + 11);
        blocked = true;
      }
    }
    if (blocked) {
      this.vx = 0;
      this.coyote = 0;
    }
  }
}

const COURSE_TOTALS: readonly [number, number] = (() => {
  let coins = 0;
  let enemies = 0;
  for (let level = 0; level < LEVELS; level++) {
    const run = Run.forLevel(level);
    coins += run.coins.length;
    enemies += run.enemies.length;
  }
  // At each handoff two course segments share the doorway coin.
  return [coins - (LEVELS - 1), enemies];
})();

export class LevelSlide {
  static readonly PAN = 1.15;
  static readonly DURATION = LevelSlide.PAN + 0.3;

  readonly previous: Run;
  time = 0;
  readonly cameraStart: number;

  constructor(previous: Run, cameraStart: number) {
    this.previous = previous;
    this.cameraStart = cameraStart;
  }

  offset(): number {
    return START_X - this.previous.destination();
  }

  camera(): number {
    const t = clamp(this.time / LevelSlide.PAN, 0, 1);
    return this.cameraStart * (1 - t * t * (3 - 2 * t));
  }

  sleeping(): boolean {
    return this.time < LevelSlide.PAN;
  }
}
