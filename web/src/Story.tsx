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
import "./story-transitions.css";
import { DownloadBadge } from "./DownloadBadge";
import {
  chapterAtProgress,
  chapterScrollTarget,
  storyProgress,
  type Chapter,
} from "./story-progress";

const chapters = [
  {
    id: "bring",
    label: "Find",
    image: bringImage,
    alt: "Three black cats discover a watercolor flower picture. The small kitten's nose and paw meet the paper on the floor.",
    status: "Find: bring an image.",
  },
  {
    id: "upload",
    label: "Send",
    image: uploadImage,
    alt: "The long paper cat and small watercolor kitten both hold the picture, lowering its bottom edge into a box with GitHub painted on its front.",
    status: "Send: upload it to GitHub.",
  },
  {
    id: "copy",
    label: "Share",
    image: copyImage,
    alt: "The flower picture lies flat inside the box, behind its raised rim. The three cats look at it together.",
    status: "Share: copy the Markdown.",
  },
] as const;

export function Story() {
  const [active, setActive] = createSignal<Chapter>(0);
  const [enhanced, setEnhanced] = createSignal(false);
  const [status, setStatus] = createSignal("");
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

      function update() {
        frame = 0;
        const pinned = window.matchMedia("(height > 500px)").matches;
        setEnhanced(pinned);
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
              story
                ?.querySelector<HTMLButtonElement>('[aria-current="step"]')
                ?.focus({ preventScroll: true });
            }
          };
          if (
            initialized &&
            !skipped &&
            !outside &&
            !fast &&
            !window.matchMedia("(prefers-reduced-motion: reduce)").matches &&
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

      update();
      window.addEventListener("scroll", schedule, { passive: true });
      window.addEventListener("resize", schedule);
      window.addEventListener("pageshow", schedule);

      return () => {
        disposed = true;
        transition?.skipTransition();
        window.cancelAnimationFrame(frame);
        window.removeEventListener("scroll", schedule);
        window.removeEventListener("resize", schedule);
        window.removeEventListener("pageshow", schedule);
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
            >
              <div {...stylex.attrs(styles.copy)}>
                {chapter.id === "bring" ? (
                  <>
                    <Heading as="h1" size="display" xstyle={styles.title} id="bring-heading">
                      Your image.
                      <br />
                      Ready to share.
                    </Heading>
                    <Text size="lead" xstyle={styles.lead}>
                      A little Mac app for sharing images in pull requests, issues, and comments.
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
                      Choose a repository. Paste or drop an image. Upload.
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
                      Copy the Markdown. Paste it where the conversation is happening.
                    </Text>
                  </>
                )}
              </div>
              <figure {...stylex.attrs(styles.scene)}>
                <img
                  {...stylex.attrs(styles.sceneImage)}
                  src={chapter.image}
                  width="1536"
                  height="1024"
                  fetchpriority={index === 0 ? "high" : "auto"}
                  alt={chapter.alt}
                />
              </figure>
            </article>
          ))}
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
                {index < 2 && <Footsteps gap={index} />}
              </>
            ))}
          </nav>
        </div>
        <VisuallyHidden as="p" id="story-status" role="status" aria-live="polite">
          {status()}
        </VisuallyHidden>
      </div>
    </section>
  );
}

function Footsteps(props: { gap: number }) {
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
});
const pawSlots = [
  density.count1,
  density.count2,
  density.count3,
  density.count4,
  density.count5,
  density.count6,
  density.count7,
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
    viewTransitionName: {
      default: null,
      [storyMedia.pinned]: "story-scene",
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

  sceneImage: {
    width: "100%",
    height: "auto",
    display: "block",
    borderRadius: radius.image,
    maxHeight: {
      default: null,
      [storyMedia.shortFlow]: 390,
      [storyMedia.pinned]: storyLayout.imageMaximum,
    },
    objectFit: "contain",
  },

  storyBottom: {
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
    borderTopWidth: { default: null, [storyMedia.pinned]: 1 },
    borderTopStyle: { default: null, [storyMedia.pinned]: "solid" },
    borderTopColor: {
      default: null,
      [storyMedia.pinned]: colors.border,
    },
  },

  chapterNav: {
    position: "relative",
    display: "grid",
    gridTemplateColumns: "auto minmax(0, 1fr) auto minmax(0, 1fr) auto",
    alignItems: "center",
    width: "100%",
    maxWidth: 720,
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
      "@container (width >= 50px)": 2,
      "@container (width >= 74px)": 3,
      "@container (width >= 98px)": 4,
      "@container (width >= 122px)": 5,
      "@container (width >= 146px)": 6,
      "@container (width >= 170px)": 7,
    },
  },
  pawSlot: { position: "relative", width: 18, height: 40, flexShrink: 0 },
  paw: (step: number, gap: number) => ({
    display: "block",
    position: "absolute",
    left: "50%",
    top: step % 2 === 0 ? 11 : 25,
    width: 12,
    height: 14,
    fill: colors.accent,
    opacity: 0.55,
    transform: `translate(-50%, -50%) rotate(${step % 2 === 0 ? 65 : 105}deg)`,
    "--paw-start": `contain calc(${gap === 0 ? 0 : 31}% + ${step + 1} * ${gap === 0 ? 30 : 37}% / (var(--paw-count) + 1))`,
    "--paw-end": `contain calc(${gap === 0 ? 2 : 33}% + ${step + 1} * ${gap === 0 ? 30 : 37}% / (var(--paw-count) + 1))`,
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
    display: "flex",
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
});
