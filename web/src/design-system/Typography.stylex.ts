import * as stylex from "@stylexjs/stylex";
import { colors, media, type } from "./tokens.stylex";
export const styles = stylex.create({
  heading: { fontFamily: type.display, fontWeight: 400, marginBlock: 0 },
  text: { marginBlock: 0 },
  muted: { color: colors.muted },
});
export const headings = stylex.create({
  display: {
    fontSize: type.displaySize,
    lineHeight: { default: 1.02, [media.phone]: 1.05 },
    letterSpacing: { default: "-0.042em", [media.phone]: "-0.04em" },
  },
  headline: {
    fontSize: type.headlineSize,
    lineHeight: { default: 1.08, [media.phone]: 1.05 },
    letterSpacing: { default: "-0.035em", [media.phone]: "-0.04em" },
  },
  title: {
    fontSize: { default: 51, [media.phone]: 43 },
    lineHeight: 1.08,
    letterSpacing: "-0.035em",
  },
  subheading: { fontSize: 27, lineHeight: 1.15, letterSpacing: "-0.025em" },
});
export const texts = stylex.create({
  body: { fontSize: { default: 18, [media.phone]: 17 } },
  lead: { fontSize: type.leadSize, lineHeight: type.leadLeading },
  small: { fontSize: 15 },
  caption: { fontSize: 13 },
});
