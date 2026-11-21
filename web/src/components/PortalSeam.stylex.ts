import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  edge: {
    position: "absolute",
    insetInline: 0,
    height: { default: 48, [media.phone]: 40 },
    pointerEvents: "none",
  },
  bottom: {
    insetBlockEnd: { default: -48, [media.phone]: -40 },
    filter: `drop-shadow(0 4px 3px ${colors.portalSeam}) drop-shadow(0 10px 10px ${colors.portalSeamAmbient})`,
  },
  top: {
    insetBlockStart: 0,
    filter: `drop-shadow(0 -4px 3px ${colors.portalSeam}) drop-shadow(0 -10px 10px ${colors.portalSeamAmbient})`,
  },
  material: {
    position: "absolute",
    inset: 0,
    height: "100%",
    maskImage: "var(--portal-seam)",
    maskRepeat: "repeat-x",
    maskSize: { default: "768px 48px", [media.phone]: "640px 40px" },
  },
  paper: {
    backgroundColor: colors.portalPaperEdge,
  },
  faceBottom: { translate: { default: "0 -2px", [media.phone]: "0 -1px" } },
  faceTop: { translate: { default: "0 2px", [media.phone]: "0 1px" } },
  clearTop: { clipPath: "inset(4px 0 0)" },
  footer: { backgroundColor: colors.canvas },
  download: { backgroundColor: colors.subtle },
});
