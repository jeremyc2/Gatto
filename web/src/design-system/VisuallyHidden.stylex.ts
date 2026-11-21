import * as stylex from "@stylexjs/stylex";
export const styles = stylex.create({
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
