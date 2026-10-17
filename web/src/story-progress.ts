export type Chapter = 0 | 1 | 2;

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
  return progress < 0.31 ? 0 : progress < 0.69 ? 1 : 2;
}

export function chapterScrollTarget(
  chapter: Chapter,
  storyTop: number,
  storyHeight: number,
  stageHeight: number,
): number {
  const positions = [0, 0.5, 0.95] as const;
  return storyTop + Math.max(1, storyHeight - stageHeight) * positions[chapter];
}
