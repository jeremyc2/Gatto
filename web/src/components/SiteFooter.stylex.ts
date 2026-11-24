import * as stylex from "@stylexjs/stylex";
import { media } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  footer: {
    position: "relative",
    zIndex: 0,
    boxSizing: "border-box",
    paddingBlockStart: 20,
    paddingBlockEnd: 27,
  },
  footerTop: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: { default: "center", [media.phone]: "start" },
    gap: 25,
  },
  actions: {
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    justifyContent: "flex-end",
    gap: { default: 20, [media.phone]: "6px 14px" },
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
