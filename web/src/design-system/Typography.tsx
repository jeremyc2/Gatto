import * as stylex from "@stylexjs/stylex";
import { dynamic, type JSX } from "@solidjs/web";
import { isStatic } from "solid-js";
import { colors, media, type } from "./tokens.stylex";

type HeadingSize = "display" | "headline" | "title" | "subheading";
type TextSize = "body" | "lead" | "small" | "caption";

export function Heading(props: {
  as?: "h1" | "h2" | "h3";
  size: HeadingSize;
  id?: string;
  children: JSX.Element;
  xstyle?: stylex.StyleXStyles;
}) {
  const Tag = dynamic(() => props.as ?? "h2", { static: isStatic(props, "as") });
  return (
    <Tag id={props.id} {...stylex.attrs(styles.heading, headings[props.size], props.xstyle)}>
      {props.children}
    </Tag>
  );
}

export function Text(props: {
  as?: "p" | "span";
  size?: TextSize;
  muted?: boolean;
  children: JSX.Element;
  xstyle?: stylex.StyleXStyles;
}) {
  const Tag = dynamic(() => props.as ?? "p", { static: isStatic(props, "as") });
  return (
    <Tag
      {...stylex.attrs(
        styles.text,
        texts[props.size ?? "body"],
        props.muted && styles.muted,
        props.xstyle,
      )}
    >
      {props.children}
    </Tag>
  );
}

const styles = stylex.create({
  heading: { fontFamily: type.display, fontWeight: 400, marginBlock: 0 },
  text: { marginBlock: 0 },
  muted: { color: colors.muted },
});
const headings = stylex.create({
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
const texts = stylex.create({
  body: { fontSize: { default: 18, [media.phone]: 17 } },
  lead: { fontSize: type.leadSize, lineHeight: type.leadLeading },
  small: { fontSize: 15 },
  caption: { fontSize: 13 },
});
