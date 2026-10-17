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
    const journey = [0, 0.44, 0.98, 1, 0.979, 0.439];
    expect(journey.map(chapterAtProgress)).toEqual([0, 1, 2, 2, 1, 0]);
  });

  test("chapter controls stop the footprint trail at the selected marker", () => {
    const progress = ([0, 1, 2] as const).map((chapter) =>
      storyProgress(chapterScrollTarget(chapter, 86, 2400, 600), 86, 2400, 600),
    );
    // The first trail finishes at 44%; the next print starts at 46%.
    expect(progress[0]).toBe(0);
    expect(progress[1]).toBeGreaterThanOrEqual(0.44);
    expect(progress[1]).toBeLessThan(0.46);
    expect(progress[2]).toBeGreaterThanOrEqual(0.98);
  });

  test("chapter controls land safely within each chapter", () => {
    for (const chapter of [0, 1, 2] as const) {
      const target = chapterScrollTarget(chapter, 86, 2400, 600);
      expect(chapterAtProgress(storyProgress(target, 86, 2400, 600))).toBe(chapter);
    }
  });

  test("Share lands at the sticky release, without an idle pinned scroll interval", () => {
    const top = 86;
    const height = 1600;
    const stage = 600;
    expect(chapterScrollTarget(2, top, height, stage)).toBe(top + height - stage);
  });
});
