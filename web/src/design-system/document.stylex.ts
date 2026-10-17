import * as stylex from "@stylexjs/stylex";
import { colors, layout, media, type } from "./tokens.stylex";

// Shared by the document and the client graph so build-time extraction includes the page canvas.
export const documentStyles = stylex.create({
  root: {
    backgroundColor: colors.canvas,
    color: colors.text,
    fontFamily: type.body,
    fontSize: 18,
    lineHeight: 1.55,
    WebkitFontSmoothing: "antialiased",
    colorScheme: { default: "light", "@media (prefers-color-scheme: dark)": "dark" },
    scrollBehavior: { default: "smooth", [media.reducedMotion]: "auto" },
    scrollPaddingTop: `calc(${layout.headerHeight} + 22px)`,
  },
  body: { margin: 0 },
});
