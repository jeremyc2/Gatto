import * as stylex from "@stylexjs/stylex";
import { colors, layout, media, space } from "../design-system/tokens.stylex";
export const styles = stylex.create({
  skip: {
    boxSizing: "border-box",
    position: "fixed",
    top: { default: -100, ":focus": 15 },
    left: 20,
    zIndex: 100,
    backgroundColor: colors.accent,
    color: colors.onAccent,
    paddingBlock: space.md,
    paddingInline: 20,
  },
  header: {
    boxSizing: "border-box",
    height: layout.headerHeight,
    position: "sticky",
    top: 0,
    zIndex: 20,
    backgroundColor: colors.canvas,
    borderBottomWidth: 1,
    borderBottomStyle: "solid",
    borderBottomColor: colors.border,
  },
  headerInner: {
    height: "100%",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 25,
  },
  nav: {
    display: "flex",
    gap: { default: space.xl, [media.tablet]: 18, [media.phone]: 10 },
    alignItems: "center",
  },
  navLink: {
    fontSize: { default: 15, [media.phone]: 14 },
    display: "inline-flex",
    minHeight: 44,
    alignItems: "center",
  },
});
