import * as stylex from "@stylexjs/stylex";
import { dynamic, type JSX } from "@solidjs/web";
import { isStatic } from "solid-js";

export function VisuallyHidden(props: {
  as?: "span" | "h1" | "h2" | "p";
  children: JSX.Element;
  id?: string;
  role?: "status";
  "aria-live"?: "polite";
}) {
  const Tag = dynamic(() => props.as ?? "span", { static: isStatic(props, "as") });
  return (
    <Tag
      id={props.id}
      role={props.role}
      aria-live={props["aria-live"]}
      {...stylex.attrs(styles.root)}
    >
      {props.children}
    </Tag>
  );
}
const styles = stylex.create({
  root: {
    position: "absolute",
    width: 1,
    height: 1,
    overflow: "hidden",
    clipPath: "inset(50%)",
    whiteSpace: "nowrap",
    margin: 0,
    padding: 0,
    borderWidth: 0,
  },
});
