import { storyTrail } from "./story-progress";

// Export settings and chapter stills share this frame clock with Aseprite's sequence.
export const storyVideo = { frames: 131, fps: 12, width: 1152, height: 768 } as const;

export function storyVideoTime(progress: number): number {
  const position = Math.max(0, Math.min(1, progress));
  const send = storyTrail[0].end;
  const share = storyTrail[1].end;
  // Keep native chapter starts aligned when the resting finale is shortened.
  const frame = Math.round(
    position <= send
      ? (position / send) * 45
      : position <= share
        ? 45 + ((position - send) / (share - send)) * 55
        : 100 + ((position - share) / (1 - share)) * (storyVideo.frames - 101),
  );
  // Seek inside a frame, avoiding rounding to its predecessor at an exact timestamp.
  return frame === 0 ? 0 : frame / storyVideo.fps + 0.001;
}

type SeekableVideo = Pick<HTMLVideoElement, "readyState" | "seeking" | "currentTime">;

// A decoder finishes its current seek before receiving the latest scroll position.
// This avoids repeatedly aborting seeks during a fast flick, including reverse scroll.
export function createStoryScrubber(video: SeekableVideo, onReady: () => void) {
  let target = 0;
  let disposed = false;
  function seek() {
    if (disposed || video.readyState < 2 || video.seeking) return;
    if (Math.abs(video.currentTime - target) > 0.5 / storyVideo.fps) {
      video.currentTime = target;
    } else onReady();
  }
  return {
    request(progress: number) {
      target = storyVideoTime(progress);
      seek();
    },
    settled() {
      if (disposed) return;
      seek();
    },
    dispose() {
      disposed = true;
    },
  };
}
