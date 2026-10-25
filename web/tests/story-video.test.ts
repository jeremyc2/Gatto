import { describe, expect, test } from "bun:test";
import { createStoryScrubber, storyVideoTime } from "../src/story-video";

describe("the scroll video", () => {
  test("seeks a restored scroll position when only metadata has loaded", () => {
    let ready = 0;
    const video = { readyState: 1, seeking: false, currentTime: 0 };
    const scrubber = createStoryScrubber(video, () => ready++);
    scrubber.request(0.69);
    expect(video.currentTime).toBe(storyVideoTime(0.69));
    expect(ready).toBe(0);
    video.readyState = 2;
    scrubber.settled();
    expect(ready).toBe(1);
  });

  test("does not reveal an undecoded initial frame or forget requests while buffering", () => {
    let ready = 0;
    const video = { readyState: 0, seeking: false, currentTime: 0 };
    const scrubber = createStoryScrubber(video, () => ready++);
    scrubber.request(0);
    video.readyState = 1;
    scrubber.settled();
    expect(ready).toBe(0);
    scrubber.request(0.8);
    video.seeking = true;
    scrubber.request(0.12);
    expect(video.currentTime).toBe(storyVideoTime(0.8));
    video.seeking = false;
    scrubber.settled();
    expect(video.currentTime).toBe(storyVideoTime(0.12));
    expect(ready).toBe(0);
    video.readyState = 3; // A later canplay can recover even without a loadeddata callback.
    scrubber.settled();
    expect(ready).toBe(1);
  });

  test("selects distinct frames within chapters, clamps endpoints and reverses", () => {
    expect(storyVideoTime(-1)).toBe(0);
    expect(storyVideoTime(2)).toBe(storyVideoTime(1));
    expect(storyVideoTime(1)).toBeCloseTo(130 / 12 + 0.001);
    expect(storyVideoTime(1)).toBeLessThan(131 / 12);
    expect(storyVideoTime(0.31)).toBeCloseTo(45 / 12 + 0.001);
    expect(storyVideoTime(0.69)).toBeCloseTo(100 / 12 + 0.001);
    const forward = [0, 0.04, 0.08, 0.12, 0.16, 0.2].map(storyVideoTime);
    expect(new Set(forward).size).toBe(6);
    expect([...forward].reverse()).toEqual([0.2, 0.16, 0.12, 0.08, 0.04, 0].map(storyVideoTime));
  });

  test("waits for data, then completes a seek before chasing the latest scroll request", () => {
    let ready = 0;
    let time = 0;
    const seeks: number[] = [];
    const video = {
      readyState: 0,
      seeking: false,
      get currentTime() {
        return time;
      },
      set currentTime(value: number) {
        time = value;
        seeks.push(value);
        this.seeking = true;
      },
    };
    const scrubber = createStoryScrubber(video, () => ready++);
    scrubber.request(0.32);
    expect(seeks).toHaveLength(0);
    video.readyState = 2;
    scrubber.settled();
    expect(seeks).toEqual([storyVideoTime(0.32)]);
    expect(ready).toBe(0); // Restored scroll must not reveal the first frame.
    scrubber.request(0.8);
    scrubber.request(0.12); // A reverse flick supersedes the unprocessed forward request.
    expect(seeks).toHaveLength(1);
    video.seeking = false;
    scrubber.settled();
    expect(seeks).toEqual([storyVideoTime(0.32), storyVideoTime(0.12)]);
    video.seeking = false;
    scrubber.settled();
    expect(ready).toBe(1);
    scrubber.request(0.12);
    expect(seeks).toHaveLength(2);
    scrubber.dispose();
    scrubber.request(1);
    scrubber.settled();
    expect(seeks).toHaveLength(2);
  });
});
