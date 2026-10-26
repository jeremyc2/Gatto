import * as stylex from "@stylexjs/stylex";
import type { JSX } from "@solidjs/web";
import { colors, radius } from "./tokens.stylex";

export function MediaFrame(props: { children: JSX.Element; xstyle?: stylex.StyleXStyles }) {
  return (
    <div {...stylex.attrs(styles.root, props.xstyle)}>
      {props.children}
      <span {...stylex.attrs(styles.edge)} aria-hidden="true" />
    </div>
  );
}

const styles = stylex.create({
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
