import * as stylex from "@stylexjs/stylex";
import { colors } from "../design-system/tokens.stylex";

export const styles = stylex.create({
  group: {
    display: { default: "inline-flex", "@media (scripting: none)": "none" },
    gap: 1,
    margin: 0,
    padding: 2,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.border,
    borderRadius: 10,
    flexShrink: 0,
  },
  option: {
    position: "relative",
    display: "grid",
    placeItems: "center",
    width: 29,
    height: 27,
    borderRadius: 7,
    color: { default: colors.muted, ":hover": colors.text },
    outline: { default: null, ":focus-within": `2px solid ${colors.focus}` },
    outlineOffset: -1,
  },
  selected: {
    backgroundColor: colors.subtle,
    color: colors.text,
  },
  input: {
    position: "absolute",
    inset: 0,
    width: "100%",
    height: "100%",
    margin: 0,
    opacity: 0,
    cursor: "pointer",
  },
});
