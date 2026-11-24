import * as stylex from "@stylexjs/stylex";
import { colors } from "./tokens.stylex";
export const styles = stylex.create({
  root: {
    color: "inherit",
    textDecoration: "underline",
    textUnderlineOffset: 5,
    touchAction: "manipulation",
    WebkitTapHighlightColor: "transparent",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 5 },
  },
  graphic: { textDecoration: "none" },
  icon: {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    minBlockSize: 44,
    minInlineSize: 44,
  },
  arrow: { display: "inline-block", whiteSpace: "nowrap", flexShrink: 0 },
});
