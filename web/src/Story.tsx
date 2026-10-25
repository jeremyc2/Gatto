import * as stylex from "@stylexjs/stylex";
import { colors, layout, media, radius } from "./design-system/tokens.stylex";
import { storyLayout, storyMedia } from "./story-layout.stylex";
import { Heading, Text } from "./design-system/Typography";
import { Button } from "./design-system/Button";
import { VisuallyHidden } from "./design-system/VisuallyHidden";
import { createEffect, createSignal, flush } from "solid-js";
import bringImage from "../assets/images/story-bring.webp";
import uploadImage from "../assets/images/story-upload.webp";
import copyImage from "../assets/images/story-copy.webp";
import journeyVideo from "../assets/video/story-journey.mp4";
import { createStoryScrubber, storyVideo } from "./story-video";
import "./story-transitions.css";
import { DownloadBadge } from "./DownloadBadge";
import {
  chapterAtProgress,
  chapterScrollTarget,
  storyProgress,
  storyTrail,
  type Chapter,
} from "./story-progress";

const chapters = [
  {
    id: "bring",
    label: "Find",
    image: bringImage,
    alt: "Three cats explore a sunlit courtyard: a watercolor kitten finds a pansy picture, a tall folded-paper cat approaches it, and a softly lit round cat investigates a shallow GitHub tray.",
    status: "Find: bring an image or video.",
  },
  {
    id: "upload",
    label: "Send",
    image: uploadImage,
    alt: "The paper cat carries the pansy picture gently in its mouth toward the GitHub tray. The watercolor kitten leads along the foreground path while the round cat watches.",
    status: "Send: upload it to GitHub.",
  },
  {
    id: "copy",
    label: "Share",
    image: copyImage,
    alt: "The pansy picture lies flat inside the GitHub tray, with the three pixel-art cats gathered around it.",
    status: "Share: copy the Markdown.",
  },
] as const;

export function Story() {
  const [active, setActive] = createSignal<Chapter>(0);
  const [enhanced, setEnhanced] = createSignal(false);
  const [status, setStatus] = createSignal("");
  const [atEnd, setAtEnd] = createSignal(false);
  const [motion, setMotion] = createSignal(false);
  const [videoReady, setVideoReady] = createSignal(false);
  const [videoFailed, setVideoFailed] = createSignal(false);
  const [copyHeight, setCopyHeight] = createSignal(0);
  const copyElements: HTMLDivElement[] = [];
  let video: HTMLVideoElement | undefined;
  let scrubber: ReturnType<typeof createStoryScrubber> | undefined;
  let story: HTMLElement | undefined;
  let stage: HTMLDivElement | undefined;
  let controlSelection = false;

  function dimensions() {
    if (!story || !stage) return undefined;
    const headerHeight = document.querySelector("header")?.offsetHeight ?? 0;
    return {
      top: story.getBoundingClientRect().top + window.scrollY - headerHeight,
      height: story.offsetHeight,
      stageHeight: stage.offsetHeight,
    };
  }

  function selectChapter(chapter: Chapter) {
    const geometry = dimensions();
    if (!geometry) return;
    controlSelection = true;
    window.scrollTo({
      top: chapterScrollTarget(chapter, geometry.top, geometry.height, geometry.stageHeight),
      behavior: "instant",
    });
  }

  createEffect(
    () => undefined,
    () => {
      let frame = 0;
      let requested: Chapter = 0;
      let transition: ViewTransition | undefined;
      let lastY = window.scrollY;
      let lastTime = performance.now();
      let disposed = false;
      let initialized = false;
      let mediaEnabled = false;
      const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
      if (video)
        scrubber = createStoryScrubber(video, () => {
          if (motion() && !videoFailed()) setVideoReady(true);
        });

      function update() {
        frame = 0;
        const pinned = window.matchMedia("(height > 500px)").matches;
        setEnhanced(pinned);
        setMotion(pinned && !reducedMotion.matches && !videoFailed());
        if (motion() !== mediaEnabled) {
          mediaEnabled = motion();
          // Source media queries are evaluated at selection, not on subsequent resizes.
          // Keep an already loading source on hydration; retry a previously excluded source.
          if (video && !videoFailed() && (!mediaEnabled || video.networkState === 3)) video.load();
        }
        if (!motion()) setVideoReady(false);
        if (!pinned) {
          transition?.skipTransition();
          initialized = false;
          return;
        }
        const geometry = dimensions();
        if (!geometry) return;
        const nextProgress = storyProgress(
          window.scrollY,
          geometry.top,
          geometry.height,
          geometry.stageHeight,
        );
        const next = chapterAtProgress(nextProgress);
        setCopyHeight(copyElements[next]?.offsetHeight ?? 0);
        if (motion()) scrubber?.request(nextProgress);
        setAtEnd(nextProgress === 1);
        const now = performance.now();
        const speed = Math.abs(window.scrollY - lastY) / Math.max(1, now - lastTime);
        lastY = window.scrollY;
        lastTime = now;
        const changed = next !== requested;
        const skipped = Math.abs(next - requested) > 1;
        requested = next;
        const outside =
          window.scrollY < geometry.top ||
          window.scrollY > geometry.top + geometry.height - geometry.stageHeight;
        const fast = speed > 1.5 && !controlSelection;
        controlSelection = false;
        if (fast || outside) transition?.skipTransition();
        if (!initialized || changed) {
          transition?.skipTransition();
          const apply = () => {
            if (disposed) return;
            // Read the latest request: skipped snapshot callbacks must never restore an old chapter.
            const hadFocus = story?.querySelector("article:focus-within");
            setActive(requested);
            setStatus(chapters[requested].status);
            flush();
            if (hadFocus && hadFocus.getAttribute("aria-hidden") === "true") {
              const control = story?.querySelector<HTMLButtonElement>('[aria-current="step"]');
              const target = control?.offsetParent
                ? control
                : story?.querySelector<HTMLElement>('article[aria-hidden="false"]');
              target?.focus({ preventScroll: true });
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
            document.documentElement.dataset.storyDirection =
              next > active() ? "forward" : "backward";
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
      for (const element of copyElements) copyObserver.observe(element);

      return () => {
        disposed = true;
        transition?.skipTransition();
        scrubber?.dispose();
        window.cancelAnimationFrame(frame);
        window.removeEventListener("scroll", schedule);
        window.removeEventListener("resize", schedule);
        window.removeEventListener("pageshow", schedule);
        reducedMotion.removeEventListener("change", schedule);
        copyObserver.disconnect();
      };
    },
  );

  return (
    <section
      {...stylex.attrs(styles.story)}
      id="journey"
      aria-label="An image finds its way to GitHub"
      ref={(element) => {
        story = element;
      }}
    >
      <div
        {...stylex.attrs(styles.stage)}
        ref={(element) => {
          stage = element;
        }}
      >
        <div {...stylex.attrs(styles.steps)}>
          {chapters.map((chapter, index) => (
            <article
              {...stylex.attrs(styles.step, active() === index && styles.activeStep)}
              id={`chapter-${chapter.id}`}
              aria-labelledby={`${chapter.id}-heading`}
              aria-hidden={enhanced() ? (active() !== index ? "true" : "false") : undefined}
              inert={enhanced() && active() !== index}
              tabindex={-1}
            >
              <div
                {...stylex.attrs(styles.copy, active() === index && styles.transitionCopy)}
                ref={(element) => {
                  copyElements[index] = element;
                }}
              >
                {chapter.id === "bring" ? (
                  <>
                    <Heading as="h1" size="display" xstyle={styles.title} id="bring-heading">
                      GitHub,
                      <br />
                      illustrated.
                    </Heading>
                    <Text size="lead" xstyle={styles.lead}>
                      A little Mac app for sharing images and videos in pull requests, issues, and
                      comments.
                    </Text>
                    <DownloadBadge />
                  </>
                ) : chapter.id === "upload" ? (
                  <>
                    <Heading size="headline" xstyle={styles.title} id="upload-heading">
                      Give it a place
                      <br />
                      on GitHub.
                    </Heading>
                    <Text size="lead" xstyle={styles.lead}>
                      Choose a repository, then drop in your files.
                    </Text>
                  </>
                ) : (
                  <>
                    <Heading size="headline" xstyle={styles.title} id="copy-heading">
                      Copy it.
                      <br />
                      Carry on.
                    </Heading>
                    <Text size="lead" xstyle={styles.lead}>
                      Paste the Markdown into your conversation.
                    </Text>
                  </>
                )}
              </div>
              <figure {...stylex.attrs(styles.scene, videoReady() && styles.coveredStill)}>
                <div {...stylex.attrs(styles.sceneFrame)}>
                  <img
                    {...stylex.attrs(styles.sceneImage)}
                    src={chapter.image}
                    width={storyVideo.width}
                    height={storyVideo.height}
                    fetchpriority={index === 0 ? "high" : "auto"}
                    alt={chapter.alt}
                  />
                  <span {...stylex.attrs(styles.sceneEdge)} aria-hidden="true" />
                </div>
              </figure>
            </article>
          ))}
          <div
            {...stylex.attrs(styles.step, styles.videoStage, videoReady() && styles.videoReady)}
            aria-hidden={videoReady() ? "false" : "true"}
          >
            <div {...stylex.attrs(styles.copy, styles.videoPlaceholder(copyHeight()))} />
            <figure {...stylex.attrs(styles.scene)} role="img" aria-label={chapters[active()].alt}>
              <div {...stylex.attrs(styles.sceneFrame)}>
                <video
                  {...stylex.attrs(styles.sceneImage)}
                  ref={(element) => {
                    video = element;
                  }}
                  poster={bringImage}
                  width={storyVideo.width}
                  height={storyVideo.height}
                  muted
                  playsinline
                  preload="auto"
                  aria-hidden="true"
                  onLoadedMetadata={() => scrubber?.settled()}
                  onLoadedData={() => scrubber?.settled()}
                  onCanPlay={() => scrubber?.settled()}
                  onSeeked={() => scrubber?.settled()}
                  onError={() => {
                    setVideoFailed(true);
                    setMotion(false);
                    setVideoReady(false);
                  }}
                >
                  <source
                    src={journeyVideo}
                    type="video/mp4"
                    media="(height > 500px) and (prefers-reduced-motion: no-preference)"
                  />
                </video>
                <span {...stylex.attrs(styles.sceneEdge)} aria-hidden="true" />
              </div>
            </figure>
          </div>
        </div>
        <div {...stylex.attrs(styles.storyBottom)}>
          <nav {...stylex.attrs(styles.chapterNav)} aria-label="Story chapters">
            {chapters.map((chapter, index) => (
              <>
                <Button
                  xstyle={[styles.chapterButton, index <= active() && styles.litChapter]}
                  variant={index <= active() ? "selected" : "quiet"}
                  aria-current={active() === index ? "step" : undefined}
                  aria-controls={`chapter-${chapter.id}`}
                  onClick={() => selectChapter(index as Chapter)}
                >
                  <span {...stylex.attrs(styles.number, index <= active() && styles.activeNumber)}>
                    {index + 1}
                  </span>
                  {chapter.label}
                </Button>
                <Footsteps gap={index as Chapter} />
              </>
            ))}
            <span
              {...stylex.attrs(styles.number, styles.endMarker, atEnd() && styles.reachedMarker)}
              aria-hidden="true"
              data-story-end=""
            >
              <svg
                {...stylex.attrs(styles.finalPaw, atEnd() && styles.reachedPaw)}
                viewBox="0 0 12 14"
              >
                <defs>
                  <linearGradient id="story-paw-rainbow" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="#f36b79" />
                    <stop offset="18%" stop-color="#f89a50" />
                    <stop offset="36%" stop-color="#e9ca54" />
                    <stop offset="52%" stop-color="#73bc87" />
                    <stop offset="68%" stop-color="#59b9d7" />
                    <stop offset="84%" stop-color="#8c8de6" />
                    <stop offset="100%" stop-color="#cd7dcc" />
                  </linearGradient>
                </defs>
                <g fill="url(#story-paw-rainbow)">
                  <ellipse cx="2" cy="5" rx="1.5" ry="2" transform="rotate(-25 2 5)" />
                  <ellipse cx="4.5" cy="2.5" rx="1.5" ry="2" transform="rotate(-8 4.5 2.5)" />
                  <ellipse cx="8" cy="2.5" rx="1.5" ry="2" transform="rotate(8 8 2.5)" />
                  <ellipse cx="10.5" cy="5" rx="1.5" ry="2" transform="rotate(25 10.5 5)" />
                  <path d="M2.5 11 C2 9 4 6.5 6 6.5 S10 9 9.5 11 C9 13 7.5 11.5 6 11.5 S3 13 2.5 11Z" />
                </g>
              </svg>
            </span>
          </nav>
        </div>
        <VisuallyHidden as="p" id="story-status" role="status" aria-live="polite">
          {status()}
        </VisuallyHidden>
      </div>
    </section>
  );
}

function Footsteps(props: { gap: Chapter }) {
  return (
    <div {...stylex.attrs(styles.route)} aria-hidden="true">
      <div {...stylex.attrs(styles.trail)}>
        {pawSlots.map((slot, step) => (
          <span {...stylex.attrs(styles.pawSlot, slot)}>
            <svg
              {...stylex.attrs(styles.paw(step, props.gap))}
              data-story-paw=""
              viewBox="0 0 12 14"
            >
              <ellipse cx="2" cy="5" rx="1.5" ry="2" transform="rotate(-25 2 5)" />
              <ellipse cx="4.5" cy="2.5" rx="1.5" ry="2" transform="rotate(-8 4.5 2.5)" />
              <ellipse cx="8" cy="2.5" rx="1.5" ry="2" transform="rotate(8 8 2.5)" />
              <ellipse cx="10.5" cy="5" rx="1.5" ry="2" transform="rotate(25 10.5 5)" />
              <path d="M2.5 11 C2 9 4 6.5 6 6.5 S10 9 9.5 11 C9 13 7.5 11.5 6 11.5 S3 13 2.5 11Z" />
            </svg>
          </span>
        ))}
      </div>
    </div>
  );
}

const density = stylex.create({
  count1: { display: "block" },
  count2: { display: { default: "none", "@container (width >= 50px)": "block" } },
  count3: { display: { default: "none", "@container (width >= 74px)": "block" } },
  count4: { display: { default: "none", "@container (width >= 98px)": "block" } },
  count5: { display: { default: "none", "@container (width >= 122px)": "block" } },
  count6: { display: { default: "none", "@container (width >= 146px)": "block" } },
  count7: { display: { default: "none", "@container (width >= 170px)": "block" } },
  count8: { display: { default: "none", "@container (width >= 194px)": "block" } },
  count9: { display: { default: "none", "@container (width >= 218px)": "block" } },
  count10: { display: { default: "none", "@container (width >= 242px)": "block" } },
  count11: { display: { default: "none", "@container (width >= 266px)": "block" } },
  count12: { display: { default: "none", "@container (width >= 290px)": "block" } },
});
const pawSlots = [
  density.count1,
  density.count2,
  density.count3,
  density.count4,
  density.count5,
  density.count6,
  density.count7,
  density.count8,
  density.count9,
  density.count10,
  density.count11,
  density.count12,
];

const leaveFootprint = stylex.keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 },
});

const styles = stylex.create({
  story: {
    position: "relative",
    viewTimelineName: "--story-route",
    viewTimelineAxis: "block",
    viewTimelineInset: `${layout.headerHeight} 0px`,
    width: layout.containerWidth,
    marginInline: "auto",
    height: {
      default: null,
      [storyMedia.pinned]: {
        // Preserve the chapter travel distances and add a visible final paw leg.
        default: "240svh",
        [media.phone]: "220svh",
      },
    },
  },

  stage: {
    position: { default: "relative", [storyMedia.pinned]: "sticky" },
    boxSizing: { default: null, [storyMedia.pinned]: "border-box" },
    top: { default: null, [storyMedia.pinned]: layout.headerHeight },
    height: {
      default: null,
      [storyMedia.pinned]: `calc(100svh - ${layout.headerHeight})`,
    },
    minHeight: {
      default: null,
      [storyMedia.pinned]: storyLayout.stageMinimum,
    },
    display: { default: null, [storyMedia.pinned]: "flex" },
    flexDirection: { default: null, [storyMedia.pinned]: "column" },
    justifyContent: { default: null, [storyMedia.pinned]: "center" },
  },

  steps: {
    position: "relative",
    height: {
      default: null,
      [storyMedia.pinned]: {
        default: "calc(100% - 85px)",
        [media.phone]: "calc(100% - 62px)",
      },
    },
    maxHeight: {
      default: null,
      [storyMedia.pinned]: storyLayout.stepsMaximum,
    },
    minHeight: {
      default: null,
      [storyMedia.pinned]: storyLayout.stepsMinimum,
    },
  },

  step: {
    boxSizing: "border-box",
    paddingBlock: {
      default: "65px",
      [storyMedia.phoneFlow]: "22px 25px",
      [storyMedia.pinned]: {
        default: "25px 15px",
        [media.phone]: "16px 10px",
        [media.compact]: "8px",
      },
    },
    display: "grid",
    gridTemplateColumns: storyLayout.columns,
    gap: {
      default: storyLayout.gap,
      [storyMedia.phoneFlow]: 10,
      [storyMedia.pinned]: storyLayout.gap,
    },
    alignItems: "center",
    alignContent: {
      default: "normal",
      [storyMedia.phoneFlow]: "start",
      [storyMedia.pinned]: {
        default: "normal",
        [media.phone]: "center",
      },
    },
    borderBottomWidth: { default: 1, [storyMedia.pinned]: 0 },
    borderBottomStyle: "solid",
    borderBottomColor: colors.border,
    position: { default: null, [storyMedia.pinned]: "absolute" },
    inset: { default: null, [storyMedia.pinned]: 0 },
    visibility: { default: null, [storyMedia.pinned]: "hidden" },
    pointerEvents: { default: null, [storyMedia.pinned]: "none" },
  },

  activeStep: {
    visibility: { default: null, [storyMedia.pinned]: "visible" },
    pointerEvents: { default: null, [storyMedia.pinned]: "auto" },
  },
  copy: {
    maxWidth: { default: 390, [media.wide]: 425, [media.phone]: "100%" },
    position: "relative",
    zIndex: 2,
    order: { default: 0, [media.phone]: 2, [media.landscape]: 1 },
    boxSizing: { default: null, [storyMedia.pinned]: "border-box" },
    paddingBlockEnd: {
      default: null,
      [storyMedia.pinned]: {
        default: 30,
        [media.phone]: 8,
        [media.compact]: 0,
      },
    },
    paddingInline: {
      default: null,
      [storyMedia.pinned]: { default: 0, [media.phone]: 4 },
    },
  },

  transitionCopy: {
    viewTransitionName: { default: null, [storyMedia.pinned]: "story-copy" },
  },

  title: {
    marginBlockStart: { default: 18, [media.phone]: 12, [media.compact]: 8 },
    marginBlockEnd: { default: 25, [media.phone]: 14, [media.compact]: 10 },
    maxWidth: { default: "none", [media.phone]: "14ch" },
  },
  lead: {
    maxWidth: { default: "31ch", [media.phone]: "32ch" },
    marginBlockEnd: {
      default: 27,
      [media.phone]: 18,
      [media.shortDesktop]: 18,
      [media.compact]: 12,
    },
  },
  scene: {
    margin: 0,
    marginInline: { default: 0, [media.phone]: "auto" },
    minWidth: 0,
    position: "relative",
    order: { default: 0, [media.phone]: 1, [media.landscape]: 2 },
    width: { default: "auto", [media.phone]: "min(100%, 530px)" },
    alignSelf: {
      default: null,
      [storyMedia.pinned]: {
        default: "auto",
        [media.phone]: "center",
      },
    },
  },

  sceneFrame: {
    position: "relative",
    width: "100%",
    marginInline: "auto",
    borderRadius: radius.media,
    overflow: "clip",
    // Keep the frame fitted to the artwork on height-constrained screens.
    maxWidth: {
      default: "100%",
      [storyMedia.shortFlow]: 585,
      [storyMedia.pinned]: `calc(${storyLayout.imageMaximum} * 1.5)`,
    },
  },
  sceneEdge: {
    position: "absolute",
    inset: 0,
    borderRadius: "inherit",
    pointerEvents: "none",
    boxShadow: `inset 0 0 0 1px ${colors.canvas}, inset 0 0 12px 2px ${colors.canvas}`,
  },
  sceneImage: {
    imageRendering: "pixelated",
    width: "100%",
    height: "auto",
    display: "block",
    maxHeight: {
      default: null,
      [storyMedia.shortFlow]: 390,
      [storyMedia.pinned]: storyLayout.imageMaximum,
    },
    objectFit: "contain",
  },

  // Share the article's exact grid and spacing so loading never moves the scene.
  videoStage: {
    display: { default: "none", [storyMedia.pinned]: "grid" },
    pointerEvents: "none",
    zIndex: 1,
  },
  videoReady: {
    visibility: { default: null, [storyMedia.pinned]: "visible" },
  },
  videoPlaceholder: (height: number) => ({ height }),
  coveredStill: {
    visibility: { default: null, [storyMedia.pinned]: "hidden" },
  },

  storyBottom: {
    containerName: "story-tracker",
    containerType: "inline-size",
    display: { default: "none", [storyMedia.pinned]: "flex" },
    boxSizing: { default: null, [storyMedia.pinned]: "border-box" },
    alignItems: { default: null, [storyMedia.pinned]: "center" },
    justifyContent: {
      default: null,
      [storyMedia.pinned]: "space-between",
    },
    minHeight: {
      default: null,
      [storyMedia.pinned]: { default: 70, [media.phone]: 62 },
    },
  },

  chapterNav: {
    position: "relative",
    display: "grid",
    gridTemplateColumns: {
      default: "repeat(3, auto minmax(0, 1fr)) auto",
      [storyMedia.compactTracker]: "repeat(3, minmax(0, 1fr)) auto",
    },
    alignItems: "center",
    width: "100%",
    maxWidth: 960,
    marginInline: "auto",
  },
  route: {
    position: "relative",
    width: "100%",
    height: 40,
    zIndex: 1,
    pointerEvents: "none",
    containerType: "inline-size",
  },
  trail: {
    display: "flex",
    boxSizing: "border-box",
    width: "100%",
    height: "100%",
    justifyContent: "space-evenly",
    gap: 6,
    paddingInline: 4,
    "--paw-count": {
      default: 1,
      "@container (50px <= width < 74px)": 2,
      "@container (74px <= width < 98px)": 3,
      "@container (98px <= width < 122px)": 4,
      "@container (122px <= width < 146px)": 5,
      "@container (146px <= width < 170px)": 6,
      "@container (170px <= width < 194px)": 7,
      "@container (194px <= width < 218px)": 8,
      "@container (218px <= width < 242px)": 9,
      "@container (242px <= width < 266px)": 10,
      "@container (266px <= width < 290px)": 11,
      "@container (width >= 290px)": 12,
    },
  },
  pawSlot: { position: "relative", width: 18, height: 40, flexShrink: 0 },
  paw: (step: number, gap: Chapter) => ({
    display: "block",
    position: "absolute",
    left: "50%",
    top: step % 2 === 0 ? 11 : 25,
    width: 12,
    height: 14,
    fill: colors.accent,
    opacity: 0.55,
    transform: `translate(-50%, -50%) rotate(${step % 2 === 0 ? 65 : 105}deg)`,
    "--paw-start": `contain calc(${storyTrail[gap].start * 100}% + ${step} * ${(storyTrail[gap].end - storyTrail[gap].start) * 100}% / var(--paw-count))`,
    "--paw-end": `contain calc(${storyTrail[gap].start * 100}% + ${step + 1} * ${(storyTrail[gap].end - storyTrail[gap].start) * 100}% / var(--paw-count))`,
    animationName: {
      default: "none",
      "@supports ((animation-timeline: --story-route) and (animation-range: contain))":
        leaveFootprint,
      [media.reducedMotion]: "none",
    },
    animationDuration: "auto",
    animationTimingFunction: "ease-out",
    animationFillMode: "both",
    animationTimeline: "--story-route",
  }),
  chapterButton: {
    backgroundColor: colors.canvas,
    color: colors.muted,
    fontSize: { default: 17, [media.phone]: 14 },
    minHeight: 48,
    paddingBlock: 5,
    paddingInline: { default: 12, [media.phone]: 0 },
    borderWidth: 0,
    borderRadius: 28,
    display: { default: "flex", [storyMedia.compactTracker]: "none" },
    gap: { default: 10, [media.phone]: 7 },
    alignItems: "center",
  },
  litChapter: {
    color: colors.text,
    backgroundColor: colors.canvas,
  },
  number: {
    boxSizing: "border-box",
    fontSize: 13,
    borderWidth: 1,
    borderStyle: "dashed",
    borderColor: colors.muted,
    borderRadius: radius.round,
    width: 30,
    height: 30,
    display: "grid",
    placeItems: "center",
  },
  activeNumber: {
    backgroundColor: colors.accent,
    color: colors.onAccent,
    borderStyle: "solid",
    borderColor: colors.accent,
    boxShadow: `0 0 0 3px ${colors.canvas}`,
  },
  endMarker: {
    width: 42,
    height: 42,
    pointerEvents: "none",
  },
  reachedMarker: {
    borderStyle: "solid",
    borderColor: colors.accent,
  },
  finalPaw: {
    display: "block",
    width: 28,
    height: 32,
    visibility: "hidden",
  },
  reachedPaw: {
    visibility: "visible",
  },
});
