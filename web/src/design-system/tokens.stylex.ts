import * as stylex from "@stylexjs/stylex";

// Themeable roles. Components consume these names rather than palette values.
export const colors = stylex.defineVars({
  canvas: { default: "#f7f3e9", "@media (prefers-color-scheme: dark)": "#1d302b" },
  text: { default: "#263e36", "@media (prefers-color-scheme: dark)": "#f2efdd" },
  muted: { default: "#58675e", "@media (prefers-color-scheme: dark)": "#bdcabb" },
  border: { default: "#d4d9c8", "@media (prefers-color-scheme: dark)": "#4e6656" },
  accent: { default: "#315e4c", "@media (prefers-color-scheme: dark)": "#bbdac0" },
  onAccent: { default: "#fffdf4", "@media (prefers-color-scheme: dark)": "#21382d" },
  subtle: { default: "#e7ecdf", "@media (prefers-color-scheme: dark)": "#30493b" },
  focus: "#a66c39",
  mediaSurface: "#0e1017",
  notificationSurface: "#fff1d0",
  notificationText: "#263e36",
  notificationBorder: "#bd965a",
  notificationShadow: "rgba(20, 35, 25, 0.22)",
  portalPaper: "#f3e5ce",
  portalSeam: "rgba(67, 44, 24, 0.32)",
  portalSeamAmbient: "rgba(67, 44, 24, 0.18)",
  portalPaperEdge: "#d6bb8f",
  portalVoid: "#000",
  portalVoidDepth: "#100910",
  portalVoidHaze: "rgba(89, 40, 17, 0.35)",
});

export const type = stylex.defineVars({
  body: "Apfel, Arial, sans-serif",
  display: "Coconat, Georgia, serif",
  displaySize: {
    default: "clamp(48px, 4.7vw, 77px)",
    "@media (760px < width <= 1050px) and (height > 700px)": "58px",
    "@media (width > 760px) and (height <= 700px)": "54px",
    "@media (width <= 760px) and (height > 620px)": "clamp(40px, 9.8vw, 55px)",
    "@media (width <= 760px) and (height <= 620px)": "30px",
  },
  headlineSize: {
    default: "clamp(45px, 4.3vw, 70px)",
    "@media (760px < width <= 1050px) and (height > 700px)": "53px",
    "@media (width > 760px) and (height <= 700px)": "50px",
    "@media (width <= 760px) and (height > 620px)": "clamp(40px, 9.8vw, 55px)",
    "@media (width <= 760px) and (height <= 620px)": "30px",
  },
  leadSize: {
    default: "20px",
    "@media (width >= 1500px) and (height > 700px)": "22px",
    "@media (760px < width <= 1050px) and (height > 700px)": "18px",
    "@media (width > 760px) and (height <= 700px)": "17px",
    "@media (width <= 760px) and (height > 620px)": "17px",
    "@media (width <= 760px) and (height <= 620px)": "14.5px",
  },
  leadLeading: {
    default: "1.5",
    "@media (width <= 760px) and (height > 620px)": "1.45",
    "@media (width <= 760px) and (height <= 620px)": "1.4",
  },
});

// Fixed scales are compile-time constants: they emit no custom properties.
export const space = stylex.defineConsts({
  xs: "4px",
  sm: "8px",
  md: "12px",
  lg: "16px",
  xl: "24px",
  xxl: "32px",
  section: "64px",
});
export const radius = stylex.defineConsts({
  image: "3px",
  control: "5px",
  media: "20px",
  round: "50%",
});
export const media = stylex.defineConsts({
  phone: "@media (width <= 760px)",
  tablet: "@media (760px < width <= 1050px)",
  wide: "@media (width >= 1500px)",
  shortDesktop: "@media (width > 760px) and (height <= 700px)",
  short: "@media (height <= 620px)",
  compact: "@media (width <= 760px) and (height <= 620px)",
  landscape: "@media (560px <= width <= 760px) and (height <= 620px)",
  reducedMotion: "@media (prefers-reduced-motion: reduce)",
});
export const layout = stylex.defineVars({
  headerHeight: { default: "86px", "@media (width <= 760px)": "73px" },
  containerWidth: {
    default: "min(1400px, calc(100% - 100px))",
    "@media (width >= 1500px)": "min(1520px, calc(100% - 140px))",
    "@media (760px < width <= 1050px)": "calc(100% - 56px)",
    "@media (width <= 760px)": "calc(100% - 36px)",
  },
});
