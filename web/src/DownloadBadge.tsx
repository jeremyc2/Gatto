import * as stylex from "@stylexjs/stylex";
import { media } from "./design-system/tokens.stylex";
import { Link } from "./design-system/Link";
import downloadBadge from "../../docs/images/download-macos.svg";

export function DownloadBadge(props: { placement?: "story" | "footer" }) {
  return (
    <Link
      href="https://github.com/jeremyc2/Gatto/releases/latest"
      variant="graphic"
      xstyle={[styles.root, props.placement === "footer" ? styles.footer : styles.story]}
    >
      <img
        {...stylex.attrs(styles.image)}
        src={downloadBadge}
        width="380"
        height="64"
        alt="Download Gatto for macOS"
      />
    </Link>
  );
}
const styles = stylex.create({
  root: {
    display: "inline-block",
    width: "min(380px, 100%)",
    maxWidth: "100%",
    verticalAlign: "top",
  },
  story: { width: { default: "min(342px, 100%)", [media.phone]: 285, [media.compact]: 250 } },
  footer: { flexShrink: 0, marginBlockStart: { default: 0, [media.phone]: 27 } },
  image: { display: "block", width: "100%", height: "auto" },
});
