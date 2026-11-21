import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  footer: {
    position: "relative",
    zIndex: 0,
    backgroundImage: {
      default: `linear-gradient(to bottom, transparent 48px, ${colors.canvas} 48px)`,
      [media.phone]: `linear-gradient(to bottom, transparent 40px, ${colors.canvas} 40px)`,
    },
    boxSizing: "border-box",
    paddingBlockStart: { default: 64, [media.phone]: 56 },
    paddingBlockEnd: 27,
  },
  footerTop: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: { default: "center", [media.phone]: "start" },
    gap: 25,
  },
  footerNav: {
    display: "flex",
    gap: { default: 23, [media.phone]: 13 },
    fontSize: 14,
    flexWrap: "wrap",
    justifyContent: { default: "normal", [media.phone]: "flex-end" },
    maxWidth: { default: "none", [media.phone]: 200 },
  },
});
