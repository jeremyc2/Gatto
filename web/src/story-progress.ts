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
  // Two transitions fill the pinned interval; Share has only a short settling edge.
  return progress < 0.44 ? 0 : progress < 0.98 ? 1 : 2;
}

export function chapterScrollTarget(
  chapter: Chapter,
  storyTop: number,
  storyHeight: number,
  stageHeight: number,
): number {
  // Send lands before its next paw trail; Share lands exactly at sticky release.
  const positions = [0, 0.45, 1] as const;
  return storyTop + Math.max(1, storyHeight - stageHeight) * positions[chapter];
}
