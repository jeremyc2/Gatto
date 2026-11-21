import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  downloadSection: {
    position: "relative",
    zIndex: 1,
    boxSizing: "border-box",
    padding: { default: "75px 0", [media.phone]: "55px 0" },
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
    backgroundColor: colors.subtle,
    textAlign: "left",
  },
  downloadRow: {
    display: "grid",
    gridTemplateColumns: {
      default: "minmax(0, 1fr) minmax(0, 1fr)",
      [media.phone]: "minmax(0, 1fr)",
    },
    alignItems: "center",
    gap: { default: 32, [media.phone]: 16 },
  },
  downloadHeading: { marginBlockEnd: 25 },
  requirements: { lineHeight: 1.7 },
  action: {
    width: "100%",
    justifySelf: "end",
    textAlign: { default: "right", [media.phone]: "left" },
  },
});
