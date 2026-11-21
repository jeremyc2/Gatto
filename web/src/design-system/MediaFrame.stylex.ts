import * as stylex from "@stylexjs/stylex";
import { colors, radius } from "./tokens.stylex";
export const styles = stylex.create({
  root: {
    position: "relative",
    width: "100%",
    borderRadius: radius.media,
    overflow: "clip",
  },
  edge: {
    position: "absolute",
    inset: 0,
    borderRadius: "inherit",
    pointerEvents: "none",
    boxShadow: `inset 0 0 0 1px ${colors.canvas}, inset 0 0 12px 2px ${colors.canvas}`,
  },
});
