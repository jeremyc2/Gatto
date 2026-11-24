import {
  chapterAtProgress,
  chapterScrollTarget,
  storyProgress,
  type Chapter,
} from "./story-progress";

type ChapterElements = {
  article: HTMLElement;
  copy: HTMLDivElement;
  control: HTMLButtonElement;
};

export type StoryElements = {
  header: HTMLElement;
  story: HTMLElement;
  stage: HTMLDivElement;
  video: HTMLVideoElement;
  chapters: readonly [ChapterElements, ChapterElements, ChapterElements];
};

/** Read the component's reactive ref collections, and wait until every ref is attached. */
export function storyElements(
  header: HTMLElement | undefined,
  story: HTMLElement | undefined,
  stage: HTMLDivElement | undefined,
  video: HTMLVideoElement | undefined,
  articles: readonly (HTMLElement | undefined)[],
  copies: readonly (HTMLDivElement | undefined)[],
  controls: readonly (HTMLButtonElement | undefined)[],
): StoryElements | undefined {
  const chapter = (index: Chapter): ChapterElements | undefined => {
    const article = articles[index];
    const copy = copies[index];
    const control = controls[index];
    return article && copy && control ? { article, copy, control } : undefined;
  };
  const first = chapter(0);
  const second = chapter(1);
  const third = chapter(2);
  if (!header || !story || !stage || !video || !first || !second || !third) return undefined;
  return { header, story, stage, video, chapters: [first, second, third] };
}

/** Read layout from component refs only after they have mounted. */
export function storyDimensions({ header, story, stage }: StoryElements) {
  return {
    top: story.getBoundingClientRect().top + window.scrollY - header.offsetHeight,
    height: story.offsetHeight,
    stageHeight: stage.offsetHeight,
  };
}

export function scrollToChapter(elements: StoryElements, chapter: Chapter) {
  const geometry = storyDimensions(elements);
  window.scrollTo({
    top: chapterScrollTarget(chapter, geometry.top, geometry.height, geometry.stageHeight),
    behavior: "instant",
  });
}

export type StoryFrame = { pinned: boolean; progress: number; copyHeight: number };

export type StoryObserver = {
  onFrame: (frame: StoryFrame) => void;
  onChapter: (chapter: Chapter) => void;
  onVideoReady: (ready: boolean) => void;
  consumeControlSelection: () => boolean;
  flush: () => void;
};

/** Own native listeners, geometry, snapshots and autoplay without creating reactive state. */
export function observeStory(elements: StoryElements, callbacks: StoryObserver) {
  let frame = 0;
  let requested: Chapter = 0;
  let transition: ViewTransition | undefined;
  let lastY = window.scrollY;
  let lastTime = performance.now();
  let disposed = false;
  let initialized = false;
  const { video } = elements;
  if (video.readyState >= 2) callbacks.onVideoReady(true);
  video.muted = true;
  void video.play().catch(() => {
    if (!disposed) callbacks.onVideoReady(false);
  });

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  function update() {
    frame = 0;
    const pinned = window.matchMedia("(height > 500px)").matches;
    if (!pinned) {
      callbacks.onFrame({ pinned: false, progress: 0, copyHeight: 0 });
      transition?.skipTransition();
      initialized = false;
      return;
    }
    const geometry = storyDimensions(elements);
    const nextProgress = storyProgress(
      window.scrollY,
      geometry.top,
      geometry.height,
      geometry.stageHeight,
    );
    const next = chapterAtProgress(nextProgress);
    callbacks.onFrame({
      pinned: true,
      progress: nextProgress,
      copyHeight: elements.chapters[next].copy.offsetHeight,
    });
    const now = performance.now();
    const speed = Math.abs(window.scrollY - lastY) / Math.max(1, now - lastTime);
    lastY = window.scrollY;
    lastTime = now;
    const previous = requested;
    const changed = next !== requested;
    const skipped = Math.abs(next - requested) > 1;
    requested = next;
    const outside =
      window.scrollY < geometry.top ||
      window.scrollY > geometry.top + geometry.height - geometry.stageHeight;
    const controlSelection = callbacks.consumeControlSelection();
    const fast = speed > 1.5 && !controlSelection;
    if (fast || outside) transition?.skipTransition();
    if (!initialized || changed) {
      transition?.skipTransition();
      const apply = () => {
        if (disposed) return;
        // Read the latest request: skipped snapshot callbacks must never restore an old chapter.
        const focusedChapter = elements.chapters.find(({ article }) =>
          article.contains(document.activeElement),
        );
        callbacks.onChapter(requested);
        callbacks.flush();
        if (focusedChapter?.article.inert) {
          const { control, article } = elements.chapters[requested];
          (control.offsetParent ? control : article).focus({ preventScroll: true });
        }
      };
      if (
        initialized &&
        !skipped &&
        !outside &&
        !fast &&
        !reducedMotion.matches &&
        document.startViewTransition
      ) {
        document.documentElement.dataset.storyDirection = next > previous ? "forward" : "backward";
        const current = document.startViewTransition(apply);
        transition = current;
        // Snapshot capture may be skipped during a fast flick or a superseding update.
        void current.ready.catch(() => undefined);
        void current.finished.then(() => {
          if (transition === current) transition = undefined;
        });
      } else apply();
    }
    initialized = true;
  }

  function schedule() {
    if (!frame) frame = window.requestAnimationFrame(update);
  }

  // Run outside the effect callback so apply() can flush before reading focus state.
  schedule();
  window.addEventListener("scroll", schedule, { passive: true });
  window.addEventListener("resize", schedule);
  window.addEventListener("pageshow", schedule);
  reducedMotion.addEventListener("change", schedule);
  // Font loading and copy wrapping also change the two-row phone composition.
  const copyObserver = new ResizeObserver(schedule);
  for (const { copy } of elements.chapters) copyObserver.observe(copy);

  return () => {
    disposed = true;
    transition?.skipTransition();
    window.cancelAnimationFrame(frame);
    window.removeEventListener("scroll", schedule);
    window.removeEventListener("resize", schedule);
    window.removeEventListener("pageshow", schedule);
    reducedMotion.removeEventListener("change", schedule);
    copyObserver.disconnect();
  };
}
