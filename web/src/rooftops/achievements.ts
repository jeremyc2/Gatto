import { timeLabel, type Score } from "./types";

export const NIGHT_EXPRESS_SECONDS = 75;

// Bit positions match the native Rust on-disk format.
export const ACHIEVEMENT_BIT = {
  CleanSweep: 0,
  Pacifist: 1,
  NightExpress: 2,
  AllNineIntact: 3,
  DoubleFeature: 4,
  BorrowedLift: 5,
  HomeByMoonlight: 6,
  EmptyPaws: 7,
} as const;

export type Achievement = keyof typeof ACHIEVEMENT_BIT;

export const ALL_ACHIEVEMENTS: readonly Achievement[] = [
  "HomeByMoonlight",
  "CleanSweep",
  "Pacifist",
  "NightExpress",
  "AllNineIntact",
  "EmptyPaws",
  "DoubleFeature",
  "BorrowedLift",
];

export function achievementLabel(award: Achievement): string {
  switch (award) {
    case "CleanSweep":
      return "Clean Sweep";
    case "Pacifist":
      return "Pacifist";
    case "NightExpress":
      return "Night Express";
    case "AllNineIntact":
      return "All Nine Intact";
    case "DoubleFeature":
      return "Double Feature";
    case "BorrowedLift":
      return "Borrowed Lift";
    case "HomeByMoonlight":
      return "Home by Moonlight";
    case "EmptyPaws":
      return "Empty Paws";
  }
}

export function achievementDescription(award: Achievement): string {
  switch (award) {
    case "CleanSweep":
      return "Finish a run with every coin collected and every enemy defeated.";
    case "Pacifist":
      return "Finish a run without defeating any enemies.";
    case "NightExpress":
      return `Finish a run in ${timeLabel(NIGHT_EXPRESS_SECONDS)} or less.`;
    case "AllNineIntact":
      return "Finish a run without dying.";
    case "DoubleFeature":
      return "Bounce from an owl or bat onto a rat in one jump.";
    case "BorrowedLift":
      return "Bounce off an enemy to the next roof without jumping again.";
    case "HomeByMoonlight":
      return "Finish all three neighborhoods.";
    case "EmptyPaws":
      return "Finish a run without collecting a single coin.";
  }
}

export function hasAchievement(mask: number, award: Achievement): boolean {
  return (mask & (1 << ACHIEVEMENT_BIT[award])) !== 0;
}

export function addAchievement(mask: number, award: Achievement): number {
  return mask | (1 << ACHIEVEMENT_BIT[award]);
}

export function countAchievements(mask: number): number {
  let count = 0;
  for (const award of ALL_ACHIEVEMENTS) {
    if (hasAchievement(mask, award)) count += 1;
  }
  return count;
}

export function recordFinishTimeAchievements(mask: number, seconds: number): number {
  let next = addAchievement(mask, "HomeByMoonlight");
  if (Math.floor(seconds) <= NIGHT_EXPRESS_SECONDS) {
    next = addAchievement(next, "NightExpress");
  }
  return next;
}

export function recordScoreAchievements(mask: number, score: Score): number {
  let next = recordFinishTimeAchievements(mask, score.seconds);
  if (score.kills === 0) next = addAchievement(next, "Pacifist");
  if (score.deaths === 0) next = addAchievement(next, "AllNineIntact");
  if (score.coins === 0) next = addAchievement(next, "EmptyPaws");
  return next;
}
