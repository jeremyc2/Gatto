import * as stylex from "@stylexjs/stylex";
import { type, media, colors } from "../design-system/tokens.stylex";
export function Brand(props: { small?: boolean; dot?: boolean }) {
  return (
    <span {...stylex.attrs(styles.root, props.small && styles.small)}>
      Gatto{props.dot && <span {...stylex.attrs(styles.dot)} aria-hidden="true" />}
    </span>
  );
}
const styles = stylex.create({
  root: {
    fontFamily: type.display,
    fontSize: { default: 40, [media.phone]: 33 },
    lineHeight: 1,
    letterSpacing: "-0.06em",
    display: "inline-flex",
    alignItems: "center",
    gap: 13,
  },
  small: { fontSize: 31 },
  dot: {
    width: 8,
    height: 8,
    backgroundColor: colors.accent,
    borderRadius: "50%",
    alignSelf: "end",
    marginBottom: 5,
    display: { default: null, [media.phone]: "none" },
  },
});
