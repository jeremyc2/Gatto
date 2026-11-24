import { recordFinishTimeAchievements, recordScoreAchievements } from "./achievements";
import { clamp, LEVELS, type Frame, type Preferences, type Replay, type Score } from "./types";

export const STORAGE_KEY = "gatto:rooftops:v2";
const FORMAT_VERSION = 2;
const SAMPLE_INTERVAL = 1 / 30;

export interface GameData {
  version: number;
  personalBest: number | null;
  replay: Replay | null;
  preferences: Preferences;
  scorecard: Score | null;
  achievements: number;
}

export function defaultPreferences(): Preferences {
  return {
    musicVolume: 75,
    effectsVolume: 55,
    ghost: true,
    animations: true,
  };
}

export function defaultGameData(): GameData {
  return {
    version: FORMAT_VERSION,
    personalBest: null,
    replay: null,
    preferences: defaultPreferences(),
    scorecard: null,
    achievements: 0,
  };
}

export function isValidScore(score: unknown): score is Score {
  if (!score || typeof score !== "object") return false;
  const s = score as Record<string, unknown>;
  return (
    typeof s.coins === "number" &&
    Number.isInteger(s.coins) &&
    s.coins >= 0 &&
    typeof s.deaths === "number" &&
    Number.isInteger(s.deaths) &&
    s.deaths >= 0 &&
    typeof s.kills === "number" &&
    Number.isInteger(s.kills) &&
    s.kills >= 0 &&
    typeof s.seconds === "number" &&
    Number.isFinite(s.seconds) &&
    s.seconds > 0 &&
    (s.personalBest === null ||
      (typeof s.personalBest === "number" && Number.isFinite(s.personalBest) && s.personalBest > 0))
  );
}

export function isValidReplay(replay: unknown): replay is Replay {
  if (!replay || typeof replay !== "object") return false;
  const r = replay as Record<string, unknown>;
  if (typeof r.seconds !== "number" || !Number.isFinite(r.seconds) || r.seconds <= 0) return false;
  if (!Array.isArray(r.frames) || r.frames.length === 0) return false;
  const frames = r.frames as unknown[];
  const first = frames[0] as Record<string, unknown> | undefined;
  const last = frames[frames.length - 1] as Record<string, unknown> | undefined;
  if (!first || !last) return false;
  if (first.seconds !== 0 || first.level !== 0) return false;
  if (last.seconds !== r.seconds || last.level !== LEVELS - 1) return false;
  const lastPose = last.pose as Record<string, unknown> | undefined;
  if (!lastPose || lastPose.home !== true) return false;

  let seenLevels = 0;
  let prevSeconds = -Infinity;
  let prevLevel = 0;
  let prevReturns = 0;
  for (const raw of frames) {
    if (!raw || typeof raw !== "object") return false;
    const f = raw as Record<string, unknown>;
    if (
      typeof f.seconds !== "number" ||
      !Number.isFinite(f.seconds) ||
      f.seconds < 0 ||
      f.seconds > r.seconds ||
      typeof f.level !== "number" ||
      !Number.isInteger(f.level) ||
      f.level < 0 ||
      f.level >= LEVELS ||
      typeof f.returns !== "number" ||
      !Number.isInteger(f.returns) ||
      f.returns < 0
    ) {
      return false;
    }
    if (f.seconds < prevSeconds || f.level < prevLevel || f.returns < prevReturns) {
      return false;
    }
    prevSeconds = f.seconds;
    prevLevel = f.level;
    prevReturns = f.returns;
    seenLevels |= 1 << f.level;

    const p = f.pose as Record<string, unknown> | undefined;
    if (!p || typeof p !== "object") return false;
    if (
      typeof p.x !== "number" ||
      typeof p.y !== "number" ||
      typeof p.facing !== "number" ||
      typeof p.gait !== "number" ||
      typeof p.squash !== "number" ||
      typeof p.home !== "boolean"
    ) {
      return false;
    }
    if (
      !Number.isFinite(p.x) ||
      !Number.isFinite(p.y) ||
      !Number.isFinite(p.facing) ||
      !Number.isFinite(p.gait) ||
      !Number.isFinite(p.squash)
    ) {
      return false;
    }
    if (Math.abs(p.facing) !== 1) return false;
    if (Math.abs(p.gait) > 3) return false;
    if (p.squash < 0 || p.squash > 1) return false;
    if (p.x < 0 || p.x > 100_000) return false;
    if (Math.abs(p.y) > 10_000) return false;
  }
  return seenLevels === (1 << LEVELS) - 1;
}

/** Query the shared total-run clock; changing levels never rewinds a ghost. */
export function frameAt(replay: Replay, seconds: number): Frame | null {
  if (!Number.isFinite(seconds) || seconds < 0 || replay.frames.length === 0) return null;
  let lo = 0;
  let hi = replay.frames.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    const f = replay.frames[mid];
    if (f && f.seconds <= seconds) lo = mid + 1;
    else hi = mid;
  }
  if (lo === 0) return null;
  const before = replay.frames[lo - 1];
  if (!before) return null;
  const after = replay.frames[lo];
  if (!after) return before;
  // Respawns and level changes are jumps, never lines through the scenery.
  if (
    before.level !== after.level ||
    before.returns !== after.returns ||
    before.pose.home !== after.pose.home ||
    after.seconds <= before.seconds
  ) {
    return before;
  }
  const t = clamp((seconds - before.seconds) / (after.seconds - before.seconds), 0, 1);
  return {
    seconds: before.seconds,
    level: before.level,
    returns: before.returns,
    pose: {
      x: before.pose.x + (after.pose.x - before.pose.x) * t,
      y: before.pose.y + (after.pose.y - before.pose.y) * t,
      facing: before.pose.facing,
      gait: before.pose.gait + (after.pose.gait - before.pose.gait) * t,
      squash: before.pose.squash + (after.pose.squash - before.pose.squash) * t,
      home: before.pose.home,
    },
  };
}

export class Recording {
  private frames: Frame[];

  constructor(initial: Frame) {
    this.frames = [initial];
  }

  capture(frame: Frame): void {
    const last = this.frames[this.frames.length - 1];
    if (
      !last ||
      frame.seconds - last.seconds >= SAMPLE_INTERVAL ||
      frame.level !== last.level ||
      frame.returns !== last.returns ||
      frame.pose.home !== last.pose.home
    ) {
      this.frames.push(frame);
    }
  }

  finish(finalFrame: Frame): Replay | null {
    this.frames.push(finalFrame);
    const replay: Replay = {
      seconds: finalFrame.seconds,
      frames: this.frames,
    };
    this.frames = [];
    return isValidReplay(replay) ? replay : null;
  }
}

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function getBrowserStorage(): StorageLike | null {
  try {
    return typeof localStorage !== "undefined" ? localStorage : null;
  } catch {
    return null;
  }
}

function parsePreferences(raw: unknown): Preferences {
  const defaults = defaultPreferences();
  if (!raw || typeof raw !== "object") return defaults;
  const p = raw as Record<string, unknown>;
  const musicVolume =
    typeof p.musicVolume === "number" && Number.isFinite(p.musicVolume)
      ? Math.round(clamp(p.musicVolume, 0, 100))
      : defaults.musicVolume;
  const effectsVolume =
    typeof p.effectsVolume === "number" && Number.isFinite(p.effectsVolume)
      ? Math.round(clamp(p.effectsVolume, 0, 100))
      : defaults.effectsVolume;
  return {
    musicVolume,
    effectsVolume,
    ghost: typeof p.ghost === "boolean" ? p.ghost : defaults.ghost,
    animations: typeof p.animations === "boolean" ? p.animations : defaults.animations,
  };
}

export function loadGameData(storage: StorageLike | null = getBrowserStorage()): GameData {
  if (!storage) return defaultGameData();
  try {
    const raw = storage.getItem(STORAGE_KEY);
    if (!raw) return defaultGameData();
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return defaultGameData();
    const data = parsed as Record<string, unknown>;
    if (data.version !== FORMAT_VERSION) return defaultGameData();

    const personalBest =
      typeof data.personalBest === "number" &&
      Number.isFinite(data.personalBest) &&
      data.personalBest > 0
        ? data.personalBest
        : null;
    let replay: Replay | null = isValidReplay(data.replay) ? data.replay : null;
    if (replay && (personalBest === null || personalBest > replay.seconds)) {
      replay = null;
    }
    const scorecard = isValidScore(data.scorecard) ? data.scorecard : null;
    let achievements =
      typeof data.achievements === "number" && Number.isInteger(data.achievements)
        ? data.achievements & 0xffff
        : 0;
    if (personalBest !== null) {
      achievements = recordFinishTimeAchievements(achievements, personalBest);
    }
    if (scorecard !== null) {
      achievements = recordScoreAchievements(achievements, scorecard);
    }
    return {
      version: FORMAT_VERSION,
      personalBest,
      replay,
      preferences: parsePreferences(data.preferences),
      scorecard,
      achievements,
    };
  } catch {
    return defaultGameData();
  }
}

export function saveGameData(
  data: GameData,
  storage: StorageLike | null = getBrowserStorage(),
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify({ ...data, version: FORMAT_VERSION }));
    return true;
  } catch {
    return false;
  }
}

export function savePreferences(
  preferences: Preferences,
  storage: StorageLike | null = getBrowserStorage(),
): boolean {
  const data = loadGameData(storage);
  data.preferences = {
    musicVolume: Math.round(clamp(preferences.musicVolume, 0, 100)),
    effectsVolume: Math.round(clamp(preferences.effectsVolume, 0, 100)),
    ghost: preferences.ghost,
    animations: preferences.animations,
  };
  return saveGameData(data, storage);
}

export function saveAchievements(
  achievements: number,
  storage: StorageLike | null = getBrowserStorage(),
): boolean {
  const data = loadGameData(storage);
  data.achievements = (data.achievements | achievements) & 0xffff;
  return saveGameData(data, storage);
}

export function saveCompletion(
  score: Score,
  replay: Replay | null,
  achievements: number,
  storage: StorageLike | null = getBrowserStorage(),
): GameData | null {
  if (!isValidScore(score)) return null;
  const data = loadGameData(storage);
  const fasterTime = data.personalBest === null || score.seconds < data.personalBest;
  if (fasterTime) {
    data.personalBest = score.seconds;
    data.scorecard = score;
  } else if (data.scorecard === null) {
    data.scorecard = score;
  }
  if (
    replay &&
    isValidReplay(replay) &&
    (data.replay === null || replay.seconds < data.replay.seconds)
  ) {
    if (data.personalBest !== null && data.personalBest <= replay.seconds) {
      data.replay = replay;
    }
  }
  data.achievements = (data.achievements | achievements) & 0xffff;
  return saveGameData(data, storage) ? data : null;
}

export function resetGameData(storage: StorageLike | null = getBrowserStorage()): void {
  if (!storage) return;
  try {
    storage.removeItem(STORAGE_KEY);
  } catch {
    // Ignore storage removal failures in restricted contexts.
  }
}
