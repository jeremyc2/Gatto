import * as stylex from "@stylexjs/stylex";

// CSS selects the first-paint layout before hydration. Short screens keep normal flow.
export const storyMedia = stylex.defineConsts({
  pinned: "@media (scripting: enabled) and (height > 500px)",
  phoneFlow:
    "@media (width <= 760px) and (scripting: none), (width <= 760px) and (height <= 500px)",
  shortFlow:
    "@media (width > 760px) and (height <= 500px), (width > 760px) and (height <= 700px) and (scripting: none)",
});

// Story geometry is local to this component, not part of the shared design language.
// Exclusive viewport ranges avoid expanding overlapping media overrides into boolean combinations.
export const storyLayout = stylex.defineVars({
  columns: {
    default: "minmax(280px, 0.88fr) minmax(0, 1.55fr)",
    "@media (width >= 1500px)": "0.8fr 1.6fr",
    "@media (760px < width <= 1050px)": "minmax(260px, 0.95fr) minmax(0, 1.3fr)",
    "@media (width <= 760px) and (height > 620px)": "1fr",
    "@media (width < 560px) and (height <= 620px)": "1fr",
    "@media (560px <= width <= 760px) and (height <= 620px)": "0.9fr 1.1fr",
  },
  gap: {
    default: "36px",
    "@media (width >= 1500px) and (height > 700px)": "65px",
    "@media (760px < width <= 1050px) and (height > 700px)": "22px",
    "@media (width > 760px) and (height <= 700px)": "30px",
    "@media (width <= 760px) and (height > 620px)": "24px",
    "@media (width < 560px) and (height <= 620px)": "12px",
    "@media (560px <= width <= 760px) and (height <= 620px)": "18px",
  },
  stageMinimum: {
    default: "570px",
    "@media (width > 760px) and (620px < height <= 700px)": "480px",
    "@media (width <= 760px)": "0px",
    "@media (height <= 620px)": "0px",
  },
  stepsMinimum: {
    default: "450px",
    "@media (width > 760px) and (620px < height <= 700px)": "380px",
    "@media (width <= 760px)": "0px",
    "@media (height <= 620px)": "0px",
  },
  stepsMaximum: {
    default: "690px",
    "@media (width >= 1500px)": "780px",
    "@media (width <= 760px)": "760px",
  },
  imageMaximum: {
    default: "none",
    "@media (width > 760px) and (620px < height <= 700px)": "390px",
    "@media (width > 760px) and (height <= 620px)": "calc(100svh - 170px)",
    "@media (width <= 760px) and (height > 620px)": "calc((100svh - 135px)*0.52)",
    "@media (width < 560px) and (height <= 620px)": "calc((100svh - 135px)*0.4)",
    "@media (560px <= width <= 760px) and (height <= 620px)": "calc(100svh - 160px)",
  },
});
