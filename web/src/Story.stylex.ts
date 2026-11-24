import * as stylex from "@stylexjs/stylex";
import { colors, layout, media, radius } from "./design-system/tokens.stylex";
import { storyLayout, storyMedia } from "./story-layout.stylex";
import { storyTrail, type Chapter } from "./story-progress";
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
export const pawSlots = [
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
].map((style, step) => ({ style, step }));
const leaveFootprint = stylex.keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 },
});
export const styles = stylex.create({
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
    marginInline: "auto",
    // Keep the frame fitted to the artwork on height-constrained screens.
    maxWidth: {
      default: "100%",
      [storyMedia.shortFlow]: 585,
      [storyMedia.pinned]: `calc(${storyLayout.imageMaximum} * 1.5)`,
    },
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
    // Hidden media can be ineligible for autoplay; the poster covers loading.
    visibility: { default: null, [storyMedia.pinned]: "visible" },
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
