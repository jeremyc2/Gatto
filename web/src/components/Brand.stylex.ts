import * as stylex from "@stylexjs/stylex";
import { type, media, colors } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  root: {
    fontFamily: type.display,
    fontSize: { default: 40, [media.phone]: 33 },
    lineHeight: 1,
    letterSpacing: "-0.06em",
    display: "inline-flex",
    alignItems: "center",
    gap: 13,
  },
  small: { fontSize: 31 },
  dot: {
    width: 8,
    height: 8,
    backgroundColor: colors.accent,
    borderRadius: "50%",
    alignSelf: "end",
    marginBottom: 5,
  },
});
