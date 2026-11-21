import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
const spin = stylex.keyframes({ to: { transform: "rotate(360deg)" } });
const float = stylex.keyframes({
  "0%, 100%": { translate: "0 1px" },
  "50%": { translate: "0 -1px" },
});
export const styles = stylex.create({
  setting: {
    backgroundColor: colors.portalPaper,
    backgroundImage: "var(--portal-paper)",
    backgroundRepeat: "repeat",
    backgroundSize: "512px 512px",
  },
  root: { overflow: "clip" },
  content: {
    boxSizing: "border-box",
    display: "flex",
    justifyContent: "center",
    paddingBlock: { default: 48, [media.phone]: 36 },
    translate: { default: "0 24px", [media.phone]: "0 20px" },
  },
  scene: {
    position: "relative",
    display: "block",
    width: "min(340px, calc(100% - 36px))",
    aspectRatio: "1",
    overflow: "clip",
    padding: 0,
    borderWidth: 0,
    borderRadius: "50%",
    backgroundColor: "transparent",
    textDecoration: "none",
    cursor: "pointer",
    touchAction: "manipulation",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 3 },
  },
  layers: { position: "absolute", inset: 0 },
  aperture: {
    position: "absolute",
    inset: "12%",
    backgroundImage: `radial-gradient(circle closest-side in oklab, ${colors.portalVoid} 0% 70%, ${colors.portalVoidDepth} 82%, ${colors.portalVoidHaze} 92%, transparent 100%)`,
  },
  ringFrame: {
    position: "absolute",
    inset: 0,
    width: "100%",
    height: "100%",
  },
  ring: {
    imageRendering: "pixelated",
    transformOrigin: "center",
    transformBox: "view-box",
    animationName: { default: spin, [media.reducedMotion]: "none" },
    animationDuration: "240s",
    animationTimingFunction: "linear",
    animationIterationCount: "infinite",
  },
  cat: {
    position: "absolute",
    imageRendering: "pixelated",
    width: "42%",
    height: "42%",
    insetInlineStart: "29%",
    insetBlockStart: "29%",
    animationName: { default: float, [media.reducedMotion]: "none" },
    animationDuration: "28s",
    animationTimingFunction: "ease-in-out",
    animationIterationCount: "infinite",
  },
});
