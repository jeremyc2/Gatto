import * as stylex from "@stylexjs/stylex";
import { colors, radius } from "./tokens.stylex";
export const styles = stylex.create({
  root: {
    boxSizing: "border-box",
    borderWidth: 0,
    backgroundColor: "transparent",
    fontFamily: "inherit",
    lineHeight: "inherit",
    color: colors.muted,
    cursor: { default: "pointer", ":disabled": "default" },
    minHeight: 44,
    borderRadius: radius.control,
    touchAction: "manipulation",
    WebkitTapHighlightColor: "transparent",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 5 },
  },
  selected: { color: colors.text, backgroundColor: colors.subtle },
});
