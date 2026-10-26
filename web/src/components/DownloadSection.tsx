import { colors, media } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Heading, Text } from "../design-system/Typography";
import { Link } from "../design-system/Link";
import { DownloadBadge } from "../DownloadBadge";
import { PortalSeam } from "./PortalSeam";
export function DownloadSection() {
  return (
    <section {...stylex.attrs(styles.downloadSection)} id="download">
      <Container xstyle={styles.downloadRow}>
        <div>
          <Heading size="title" xstyle={styles.downloadHeading}>
            Free.
            <br />
            Open source.
          </Heading>
          <Text size="small" muted xstyle={styles.requirements}>
            Apple Silicon · macOS 13 or later
            <br />
            GitHub CLI required.{" "}
            <Link href="https://github.com/jeremyc2/Gatto/blob/main/docs/SETUP.md">
              Setup guide
            </Link>
          </Text>
        </div>
        <div {...stylex.attrs(styles.action)}>
          <DownloadBadge placement="footer" />
        </div>
      </Container>
      <PortalSeam edge="bottom" />
    </section>
  );
}
const styles = stylex.create({
  downloadSection: {
    position: "relative",
    zIndex: 1,
    boxSizing: "border-box",
    padding: { default: "75px 0", [media.phone]: "55px 0" },
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
    backgroundColor: colors.subtle,
    textAlign: "left",
  },
  downloadRow: {
    display: "grid",
    gridTemplateColumns: {
      default: "minmax(0, 1fr) minmax(0, 1fr)",
      [media.phone]: "minmax(0, 1fr)",
    },
    alignItems: "center",
    gap: { default: 32, [media.phone]: 16 },
  },
  downloadHeading: { marginBlockEnd: 25 },
  requirements: { lineHeight: 1.7 },
  action: {
    width: "100%",
    justifySelf: "end",
    textAlign: { default: "right", [media.phone]: "left" },
  },
});
