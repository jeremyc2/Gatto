export type Chapter = 0 | 1 | 2;

// Each leg ends at a marker. The final leg holds Share until sticky release.
export const storyTrail = [
  { start: 0.02, end: 0.31 },
  { start: 0.33, end: 0.69 },
  { start: 0.71, end: 1 },
] as const;

export function storyProgress(
  scrollY: number,
  storyTop: number,
  storyHeight: number,
  stageHeight: number,
): number {
  const range = Math.max(1, storyHeight - stageHeight);
  return Math.max(0, Math.min(1, (scrollY - storyTop) / range));
}

export function chapterAtProgress(progress: number): Chapter {
  return progress < storyTrail[0].end ? 0 : progress < storyTrail[1].end ? 1 : 2;
}

export function chapterScrollTarget(
  chapter: Chapter,
  storyTop: number,
  storyHeight: number,
  stageHeight: number,
): number {
  // Controls land at their marker, before the next leg starts.
  const positions = [0, 0.32, 0.7] as const;
  return storyTop + Math.max(1, storyHeight - stageHeight) * positions[chapter];
}
