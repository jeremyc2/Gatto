import * as stylex from "@stylexjs/stylex";
import type { JSX } from "@solidjs/web";
import { colors } from "./tokens.stylex";
import { VisuallyHidden } from "./VisuallyHidden";

export function Link(props: {
  href: string;
  children: JSX.Element;
  label?: string;
  variant?: "text" | "graphic" | "icon";
  xstyle?: stylex.StyleXStyles;
}) {
  const external = () => /^(?:https?:)?\/\//i.test(props.href);
  return (
    <a
      href={props.href}
      target={external() ? "_blank" : undefined}
      rel={external() ? "noopener noreferrer" : undefined}
      aria-label={props.label && external() ? `${props.label} (opens in a new tab)` : props.label}
      {...stylex.attrs(
        styles.root,
        props.variant && props.variant !== "text" && styles.graphic,
        props.variant === "icon" && styles.icon,
        props.xstyle,
      )}
    >
      {props.children}
      {!props.href.startsWith("#") && (!props.variant || props.variant === "text") && (
        <span aria-hidden="true" {...stylex.attrs(styles.arrow)}>
          {"\u00a0↗"}
        </span>
      )}
      {external() && !props.label && <VisuallyHidden> (opens in a new tab)</VisuallyHidden>}
    </a>
  );
}
const styles = stylex.create({
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
