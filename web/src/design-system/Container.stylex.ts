import * as stylex from "@stylexjs/stylex";
import { layout } from "./tokens.stylex";
export const styles = stylex.create({
  root: { width: layout.containerWidth, marginInline: "auto" },
});
