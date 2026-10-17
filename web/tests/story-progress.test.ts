import { describe, expect, test } from "bun:test";
import { chapterAtProgress, chapterScrollTarget, storyProgress } from "../src/story-progress";

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
    const journey = [0, 0.31, 0.69, 1, 0.68, 0.3];
    expect(journey.map(chapterAtProgress)).toEqual([0, 1, 2, 2, 1, 0]);
  });

  test("chapter controls land safely within each chapter", () => {
    for (const chapter of [0, 1, 2] as const) {
      const target = chapterScrollTarget(chapter, 86, 2400, 600);
      expect(chapterAtProgress(storyProgress(target, 86, 2400, 600))).toBe(chapter);
    }
  });
});
