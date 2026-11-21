import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  details: {
    boxSizing: "border-box",
    padding: { default: "110px 0 115px", [media.phone]: "65px 0" },
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
  },
  extraGrid: {
    display: "grid",
    gridTemplateColumns: { default: "repeat(3, minmax(0, 1fr))", [media.phone]: "1fr" },
    gap: { default: 45, [media.phone]: 40 },
    marginTop: 0,
  },
  feature: {
    boxSizing: "border-box",
    minWidth: 0,
    width: "100%",
    maxWidth: { default: "none", [media.phone]: 530 },
    marginInline: { default: 0, [media.phone]: "auto" },
  },
  screenshotFrame: {
    aspectRatio: "4 / 3",
    marginBlockEnd: 24,
    backgroundColor: colors.mediaSurface,
  },
  screenshot: {
    position: "absolute",
    inset: 0,
    display: "block",
    width: "100%",
    height: "100%",
    objectFit: "contain",
  },
  featureHeading: { marginBlockEnd: 14 },
  featureDescription: { maxWidth: { default: "29ch", [media.phone]: "38ch" }, marginBlockEnd: 13 },
});
