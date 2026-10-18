import { describe, expect, test } from "bun:test";
import {
  chapterAtProgress,
  chapterScrollTarget,
  storyProgress,
  storyTrail,
} from "../src/story-progress";

describe("the pinned story", () => {
  test("clamps to the first and last scenes outside the story", () => {
    expect(storyProgress(-100, 0, 2400, 600)).toBe(0);
    expect(storyProgress(2200, 0, 2400, 600)).toBe(1);
  });

  test("uses the pinned range, including the header-adjusted story origin", () => {
    expect(storyProgress(1100, 200, 2400, 600)).toBe(0.5);
    expect(storyProgress(200, 200, 600, 600)).toBe(0);
  });

  test("reverses cleanly when scrolling back through chapter boundaries", () => {
    const journey = [0, 0.31, 0.69, 0.85, 1, 0.85, 0.69, 0.689, 0.309];
    expect(journey.map(chapterAtProgress)).toEqual([0, 1, 2, 2, 2, 2, 2, 1, 0]);
  });

  test("chapter controls stop the footprint trail at the selected marker", () => {
    const progress = ([0, 1, 2] as const).map((chapter) =>
      storyProgress(chapterScrollTarget(chapter, 86, 2400, 600), 86, 2400, 600),
    );
    expect(progress[0]).toBe(0);
    expect(progress[1]).toBeGreaterThanOrEqual(storyTrail[0].end);
    expect(progress[1]).toBeLessThan(storyTrail[1].start);
    expect(progress[2]).toBeGreaterThanOrEqual(storyTrail[1].end);
    expect(progress[2]).toBeLessThan(storyTrail[2].start);
  });

  test("chapter controls land safely within each chapter", () => {
    for (const chapter of [0, 1, 2] as const) {
      const target = chapterScrollTarget(chapter, 86, 2400, 600);
      expect(chapterAtProgress(storyProgress(target, 86, 2400, 600))).toBe(chapter);
    }
  });

  test("Share leaves room for the final paw trail before sticky release", () => {
    const top = 86;
    const height = 1600;
    const stage = 600;
    const target = chapterScrollTarget(2, top, height, stage);
    const release = top + height - stage;
    expect(target).toBeLessThan(release);
    expect(chapterAtProgress(storyProgress(target, top, height, stage))).toBe(2);
    expect(storyTrail[2].end).toBe(1);
    // Returning through the final leg keeps Share until its own marker is crossed.
    expect(chapterAtProgress(storyProgress(release - 100, top, height, stage))).toBe(2);
  });
});
