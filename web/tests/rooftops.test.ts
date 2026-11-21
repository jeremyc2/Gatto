import { describe, expect, test } from "bun:test";
import { hasAchievement } from "../src/rooftops/achievements";
import { posterText } from "../src/rooftops/art";
import { Death } from "../src/rooftops/audio/death";
import {
  CUE_COIN,
  CUE_RETURN,
  CUE_SHUTTERS,
  FINALE,
  MENU,
  Synth,
} from "../src/rooftops/audio/synth";
import { computeCourseViewport, computeTouchControlsLayout } from "../src/rooftops/course-renderer";
import { Flight, menuTitleLayout, moonProjection } from "../src/rooftops/harbor/flight";
import {
  Camera,
  defaultAim,
  HARBOR_HEIGHT,
  HARBOR_WIDTH,
  HORIZON,
  View,
} from "../src/rooftops/harbor/projection";
import {
  courseProgress,
  enemyPosition,
  ghostGap,
  levelRoofs,
  LevelSlide,
  Run,
} from "../src/rooftops/simulation";
import {
  defaultPreferences,
  frameAt,
  loadGameData,
  Recording,
  resetGameData,
  saveAchievements,
  saveCompletion,
  savePreferences,
  STORAGE_KEY,
  type StorageLike,
} from "../src/rooftops/storage";
import { COURSE_MOON_DIAMETER, START_X, timeLabel, type Score } from "../src/rooftops/types";

function createMemoryStorage(): StorageLike {
  const map = new Map<string, string>();
  return {
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => {
      map.set(key, value);
    },
    removeItem: (key) => {
      map.delete(key);
    },
  };
}

describe("Rooftops simulation", () => {
  test("builds 17, 19, and 21 roofs across Brownstones, SoHo, and Midtown", () => {
    expect(levelRoofs(0)).toHaveLength(17);
    expect(levelRoofs(1)).toHaveLength(19);
    expect(levelRoofs(2)).toHaveLength(21);
    expect(courseProgress(0, START_X)).toBe(0);
    expect(courseProgress(1, START_X)).toBeGreaterThan(courseProgress(0, 1000));
    expect(courseProgress(2, START_X)).toBeGreaterThan(courseProgress(1, START_X));
  });

  test("supports variable-height jumps, enemy stomps, Double Feature, and Borrowed Lift", () => {
    const run = Run.forLevel(1);
    run.jumpDown();
    run.step();
    const afterStepVy = -535 + 1450 / 120;
    expect(run.vy).toBeCloseTo(afterStepVy, 4);
    run.jumpUp();
    expect(run.vy).toBeCloseTo(afterStepVy * 0.55, 2);

    // Position the cat falling onto a flying bat, then bouncing onto a rat on the next roof.
    const bat = run.enemies.find((e) => e.kind === "bat")!;
    const [bx, by] = enemyPosition(bat, run.roofs, run.time + 1 / 120);
    run.x = bx;
    run.y = by - 14;
    run.vy = 200;
    run.ground = null;
    run.jumpDown();
    run.ground = 0;
    run.coyote = 0.12;
    run.step(); // start jump chain
    const [bx2, by2] = enemyPosition(bat, run.roofs, run.time + 1 / 120);
    run.x = bx2;
    run.y = by2 - 12;
    run.vy = 220;
    run.ground = null;
    run.step();
    expect(bat.defeated).toBe(true);
    expect(run.vy).toBeLessThan(-300);

    // Bounce onto a rat without landing in between -> Double Feature.
    const rat = run.enemies.find((e) => e.kind === "rat")!;
    const [rx, ry] = enemyPosition(rat, run.roofs, run.time + 1 / 120);
    run.x = rx;
    run.y = ry - 12;
    run.vy = 220;
    run.ground = null;
    run.step();
    expect(rat.defeated).toBe(true);
    expect(hasAchievement(run.stunts.unlocked, "DoubleFeature")).toBe(true);

    // Land on the next roof after stomping the rat -> Borrowed Lift.
    const nextRoof = run.roofs[rat.roof + 1]!;
    run.x = nextRoof.x + 24;
    run.y = nextRoof.y - 1;
    run.vy = 220;
    run.ground = null;
    run.step();
    const groundAfter = (r: Run) => r.ground;
    expect(groundAfter(run)).toBe(rat.roof + 1);
    expect(hasAchievement(run.stunts.unlocked, "BorrowedLift")).toBe(true);
  });

  test("grants 1.2s of invincibility during respawn flash so enemies cannot immediately kill the cat", () => {
    const run = Run.forLevel(0);
    run.enemies = [{ roof: 0, kind: "rat", defeated: false, phase: 0, speed: 0 }];
    run.respawn();
    expect(run.returns).toBe(1);
    expect(run.invulnerable).toBeCloseTo(1.2, 4);

    // Overlap the rat directly throughout the entire flash window: cannot die while flashing.
    for (let i = 0; i < 140; i++) {
      const [ex, ey] = enemyPosition(run.enemies[0]!, run.roofs, run.time + 1 / 120);
      run.x = ex;
      run.y = ey + 11;
      run.step();
    }
    expect(run.returns).toBe(1);
    expect(run.invulnerable).toBeGreaterThan(0);

    // Once invincibility expires (after 1.2s = 144 steps), enemy contact respawns again.
    for (let i = 0; i < 10; i++) {
      const [ex, ey] = enemyPosition(run.enemies[0]!, run.roofs, run.time + 1 / 120);
      run.x = ex;
      run.y = ey + 11;
      run.step();
    }
    expect(run.returns).toBe(2);
  });

  test("deduplicates the shared doorway coin across level handoffs and evaluates slide camera", () => {
    const run = Run.forLevel(0);
    const lastCoin = run.coins[run.coins.length - 1]!;
    lastCoin.collected = true;
    const prevDest = run.destination();
    run.home = true;
    run.jumpDown();
    expect(run.level).toBe(1);
    expect(run.bankedCoins).toBe(1);
    expect(run.coins).toHaveLength(levelRoofs(1).length - 1);

    const slide = new LevelSlide(Run.forLevel(0), -400);
    expect(slide.offset()).toBe(START_X - prevDest);
    expect(slide.sleeping()).toBe(true);
    slide.time = LevelSlide.PAN;
    expect(slide.sleeping()).toBe(false);
    expect(slide.camera()).toBeCloseTo(0, 5);
  });

  test("checkpoint shelter coins clear the building coping at mobile scale and remain jump-reachable", () => {
    const mobileCoinRadius = 35 * 0.5;
    for (let level = 0; level < 3; level++) {
      const run = Run.forLevel(level);
      const lastRoof = run.roofs[run.roofs.length - 1]!;
      const lastCoin = run.coins[run.coins.length - 1]!;
      // Checkpoint coping sits at roof.y - 92; bottom edge of mobile coin stays strictly above it.
      expect(lastCoin.y + mobileCoinRadius).toBeLessThan(lastRoof.y - 92);
      if (level > 0) {
        const startRoof = run.roofs[0]!;
        const startCoin = run.coins[0]!;
        expect(startCoin.y).toBe(lastCoin.y);
        expect(startCoin.y + mobileCoinRadius).toBeLessThan(startRoof.y - 92);
      }

      // Standing directly below the checkpoint coin and jumping collects it cleanly.
      run.x = lastCoin.x;
      run.y = lastRoof.y;
      run.ground = run.roofs.length - 1;
      run.jumpDown();
      for (let i = 0; i < 60; i++) run.step();
      expect(lastCoin.collected).toBe(true);
    }
  });

  test("computes offscreen ghost gap across level boundaries", () => {
    const player = Run.forLevel(1);
    player.x = START_X + 128;
    const ghost = Run.forLevel(0).ghostFrame();
    const gap = ghostGap(player, ghost, 0, 900);
    expect(gap).not.toBeNull();
    expect(gap!.ahead).toBe(true);
    expect(gap!.meters).toBeGreaterThan(2);
  });
});

describe("Rooftops localStorage persistence and replay", () => {
  test("saves preferences, achievements, completions, and validates ghost replays", () => {
    const storage = createMemoryStorage();
    expect(loadGameData(storage).preferences).toEqual(defaultPreferences());

    expect(
      savePreferences(
        { musicVolume: 40, effectsVolume: 0, ghost: false, animations: false },
        storage,
      ),
    ).toBe(true);
    expect(loadGameData(storage).preferences).toEqual({
      musicVolume: 40,
      effectsVolume: 0,
      ghost: false,
      animations: false,
    });

    const r0 = Run.forLevel(0);
    const rec = new Recording(r0.ghostFrame());
    r0.elapsed = 20;
    r0.x = 500;
    rec.capture(r0.ghostFrame());
    const r1 = Run.forLevel(1);
    r1.elapsed = 40;
    rec.capture(r1.ghostFrame());
    const r2 = Run.forLevel(2);
    r2.elapsed = 62.5;
    r2.x = r2.destination();
    r2.home = true;
    const replay = rec.finish(r2.ghostFrame());
    expect(replay).not.toBeNull();

    // Interpolates smoothly within a level, jumps cleanly across level boundary.
    const mid = frameAt(replay!, 10)!;
    expect(mid.level).toBe(0);
    expect(mid.pose.x).toBeCloseTo((START_X + 500) * 0.5, 2);
    const boundary = frameAt(replay!, 30)!;
    expect(boundary.level).toBe(0);
    expect(boundary.pose.x).toBe(500);

    const score: Score = {
      coins: 12,
      deaths: 0,
      kills: 0,
      seconds: 62.5,
      personalBest: null,
    };
    const saved = saveCompletion(score, replay, r2.completionAchievements(score), storage);
    expect(saved).not.toBeNull();
    const loaded = loadGameData(storage);
    expect(loaded.personalBest).toBe(62.5);
    expect(loaded.scorecard?.seconds).toBe(62.5);
    expect(loaded.replay?.seconds).toBe(62.5);
    expect(hasAchievement(loaded.achievements, "HomeByMoonlight")).toBe(true);
    expect(hasAchievement(loaded.achievements, "NightExpress")).toBe(true);
    expect(hasAchievement(loaded.achievements, "Pacifist")).toBe(true);
    expect(hasAchievement(loaded.achievements, "AllNineIntact")).toBe(true);

    // Slower run keeps faster personal best and replay.
    const slower: Score = { ...score, seconds: 90, personalBest: 62.5 };
    saveCompletion(slower, null, 0, storage);
    expect(loadGameData(storage).personalBest).toBe(62.5);

    saveAchievements(1 << 4, storage);
    expect(hasAchievement(loadGameData(storage).achievements, "DoubleFeature")).toBe(true);

    // Corrupt JSON recovers cleanly to defaults.
    storage.setItem(STORAGE_KEY, "{not-valid-json");
    expect(loadGameData(storage).personalBest).toBeNull();

    resetGameData(storage);
    expect(storage.getItem(STORAGE_KEY)).toBeNull();
  });

  test("shows personal-best footer only for slower runs and formats hours", () => {
    const hasFooter = (score: Score) =>
      posterText(score).find((item) => item.label.startsWith("personal best:"))?.label ?? null;
    expect(
      hasFooter({ coins: 0, deaths: 0, kills: 0, seconds: 120, personalBest: null }),
    ).toBeNull();
    expect(
      hasFooter({ coins: 0, deaths: 0, kills: 0, seconds: 120, personalBest: 120 }),
    ).toBeNull();
    expect(
      hasFooter({ coins: 0, deaths: 0, kills: 0, seconds: 119, personalBest: 120 }),
    ).toBeNull();
    expect(hasFooter({ coins: 0, deaths: 0, kills: 0, seconds: 121, personalBest: 120 })).toBe(
      "personal best: 2:00",
    );
    expect(timeLabel(125.9)).toBe("2:05");
    expect(timeLabel(3661)).toBe("1:01:01");
  });
});

describe("Rooftops procedural audio synthesizer", () => {
  test("renders finite, bounded stereo output across all 5 scenes and adapts to 44.1 kHz", () => {
    for (const rate of [48_000, 44_100]) {
      const synth = new Synth(42, rate);
      let peak = 0;
      let diff = 0;
      for (const level of [0, 1, 2, FINALE, MENU]) {
        synth.scene(level, level === FINALE);
        synth.thruster(level === 0);
        synth.cues(CUE_COIN | CUE_RETURN | CUE_SHUTTERS);
        for (let i = 0; i < Math.round(rate * 3.5); i++) {
          const [l, r] = synth.next(true, 100, 100);
          expect(Number.isFinite(l) && Number.isFinite(r)).toBe(true);
          peak = Math.max(peak, Math.abs(l), Math.abs(r));
          diff += (l - r) * (l - r);
        }
        expect(synth.level).toBe(level);
      }
      expect(peak).toBeGreaterThan(0.05);
      expect(peak).toBeLessThanOrEqual(0.95);
      expect(diff).toBeGreaterThan(0.05);
    }
  });

  test("freezes musical clock when suspended and keeps effects preview independent", () => {
    const synth = new Synth(7, 48_000);
    for (let i = 0; i < 24_000; i++) synth.next(true, 100, 100);
    for (let i = 0; i < 6_000; i++) synth.next(false, 100, 100);
    const frozenSamples = synth.samples;
    for (let i = 0; i < 2_000; i++) {
      const [l, r] = synth.next(false, 100, 100);
      expect(l).toBe(0);
      expect(r).toBe(0);
    }
    expect(synth.samples).toBe(frozenSamples);

    synth.previewEffects(100);
    let previewPeak = 0;
    for (let i = 0; i < 7200; i++) {
      previewPeak = Math.max(previewPeak, Math.abs(synth.previewSample()));
    }
    expect(previewPeak).toBeGreaterThan(0.15);
    expect(previewPeak).toBeLessThanOrEqual(0.16);
    expect(synth.previewing()).toBe(false);
    expect(synth.samples).toBe(frozenSamples);
  });

  test("death patch produces sustained low rumble and terminates cleanly", () => {
    const death = new Death(48_000);
    death.trigger();
    let tail = 0;
    for (let frame = 0; frame < 48_000 * 4; frame++) {
      const s = death.next();
      expect(Number.isFinite(s) && Math.abs(s) < 0.1).toBe(true);
      if (frame >= 48_000 && frame < 96_000) tail = Math.max(tail, Math.abs(s));
      if (frame > 173_000) expect(s).toBe(0);
    }
    expect(tail).toBeGreaterThan(0.01);
  });
});

describe("Rooftops harbor 3D projection and fly-through", () => {
  test("Camera.atMenu matches View.project and moon grows to 130% during flight", () => {
    const view = new View(HARBOR_WIDTH, HARBOR_HEIGHT * HORIZON, defaultAim());
    const rest = Camera.atMenu(view);
    for (const [x, y, z] of [
      [200, 240, 0],
      [600, 400, -250],
      [720, 110, 270],
    ] as const) {
      const expected = view.project(x, z, y);
      const actual = rest.point([x, y, z])!;
      expect(Math.abs(expected[0] - actual[0])).toBeLessThan(0.001);
      expect(Math.abs(expected[1] - actual[1])).toBeLessThan(0.001);
    }

    const flight = new Flight();
    flight.time = Flight.SHOT_END;
    const endCam = flight.camera(view);
    const proj = moonProjection(view, endCam)!;
    expect(proj[1] * 64).toBeCloseTo(COURSE_MOON_DIAMETER * 1.3, 2);
  });

  test("course viewport zooms in for mobile portrait/landscape while preserving desktop framing", () => {
    const desktop = computeCourseViewport(1280, 720);
    expect(desktop.verticalOffset).toBe(0);
    expect(desktop.viewH).toBeCloseTo(560, 2);

    const landscape = computeCourseViewport(844, 390);
    expect(landscape.scale).toBeGreaterThan(390 / 560);
    expect(landscape.viewW).toBeGreaterThan(800);

    const portrait = computeCourseViewport(390, 844);
    expect(portrait.scale).toBeGreaterThan(0.75);
    expect(portrait.verticalOffset).toBeGreaterThan(100);

    const touch = computeTouchControlsLayout(844, 390, 20, 44, 44);
    expect(touch.joyRadius).toBe(touch.jumpRadius);
    expect(touch.joyBaseY).toBe(touch.jumpY);
    expect(touch.joyBaseX - touch.joyRadius).toBeGreaterThanOrEqual(44);
    expect(touch.jumpX + touch.jumpRadius).toBeLessThanOrEqual(844 - 44);
    expect(touch.joyBaseY + touch.joyRadius).toBeLessThanOrEqual(390 - 20);
  });

  test("touch controls and menu layout stay within safe areas across modern iPhone and Android sizes", () => {
    const mobileLandscapes = [
      // [width, height, safeBottom, safeLeft, safeRight]
      [667, 375, 0, 0, 0], // iPhone SE
      [667, 318, 0, 0, 0], // iPhone SE Safari chrome
      [852, 393, 21, 59, 59], // iPhone 15 / 16 Pro
      [852, 330, 21, 59, 59], // iPhone 15 / 16 Pro Safari chrome
      [956, 440, 21, 59, 59], // iPhone 16 Pro Max
      [915, 412, 0, 24, 24], // Pixel 9 / Galaxy S24
      [740, 360, 0, 20, 20], // Compact Android
      [740, 300, 0, 20, 20], // Compact Android Chrome bar
    ] as const;

    for (const [w, h, safeB, safeL, safeR] of mobileLandscapes) {
      const touch = computeTouchControlsLayout(w, h, safeB, safeL, safeR);
      expect(touch.joyRadius).toBe(touch.jumpRadius);
      expect(touch.joyBaseY).toBe(touch.jumpY);
      expect(touch.joyRadius).toBeGreaterThanOrEqual(42);
      expect(touch.joyBaseX - touch.joyRadius).toBeGreaterThanOrEqual(safeL);
      expect(touch.jumpX + touch.jumpRadius).toBeLessThanOrEqual(w - safeR);
      expect(touch.joyBaseY + touch.joyRadius).toBeLessThanOrEqual(h - safeB);
      // Center lane between controls stays wide enough for caption/doorway pills.
      expect(touch.jumpX - touch.jumpRadius - (touch.joyBaseX + touch.joyRadius)).toBeGreaterThan(
        260,
      );

      const menu = menuTitleLayout(w, h, true);
      expect(menu.actionsTop).toBeGreaterThan(menu.titleY);
      // Four 40px buttons + three 7px gaps = 181px; must fit above safe-bottom without clipping.
      expect(menu.actionsTop + 181).toBeLessThanOrEqual(h - safeB);
    }
  });
});
