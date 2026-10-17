import * as stylex from "@stylexjs/stylex";
import type { JSX } from "@solidjs/web";
import { colors } from "./tokens.stylex";

export function Link(props: {
  href: string;
  children: JSX.Element;
  label?: string;
  xstyle?: stylex.StyleXStyles;
}) {
  return (
    <a href={props.href} aria-label={props.label} {...stylex.attrs(styles.root, props.xstyle)}>
      {props.children}
    </a>
  );
}
const styles = stylex.create({
  root: {
    color: "inherit",
    textUnderlineOffset: 5,
    touchAction: "manipulation",
    WebkitTapHighlightColor: "transparent",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 5 },
  },
});
