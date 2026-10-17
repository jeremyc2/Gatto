import * as stylex from "@stylexjs/stylex";
import type { JSX } from "@solidjs/web";
import { colors, radius } from "./tokens.stylex";

export function Button(props: {
  children: JSX.Element;
  onClick: () => void;
  variant?: "quiet" | "selected";
  disabled?: boolean;
  "aria-current"?: "step" | undefined;
  "aria-controls"?: string;
  xstyle?: stylex.StyleXStyles;
}) {
  return (
    <button
      type="button"
      disabled={props.disabled}
      onClick={props.onClick}
      aria-current={props["aria-current"]}
      aria-controls={props["aria-controls"]}
      {...stylex.attrs(styles.root, props.variant === "selected" && styles.selected, props.xstyle)}
    >
      {props.children}
    </button>
  );
}
const styles = stylex.create({
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
