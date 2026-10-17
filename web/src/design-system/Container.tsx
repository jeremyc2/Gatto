import * as stylex from "@stylexjs/stylex";
import type { JSX } from "@solidjs/web";
import { layout } from "./tokens.stylex";

export function Container(props: { children: JSX.Element; xstyle?: stylex.StyleXStyles }) {
  return <div {...stylex.attrs(styles.root, props.xstyle)}>{props.children}</div>;
}

const styles = stylex.create({
  root: { width: layout.containerWidth, marginInline: "auto" },
});
