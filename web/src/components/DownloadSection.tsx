import { colors, media } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Heading, Text } from "../design-system/Typography";
import { Link } from "../design-system/Link";
import { DownloadBadge } from "../DownloadBadge";
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
              Setup guide ↗
            </Link>
          </Text>
        </div>
        <DownloadBadge placement="footer" />
      </Container>
    </section>
  );
}
const styles = stylex.create({
  downloadSection: {
    boxSizing: "border-box",
    padding: { default: "75px 0", [media.phone]: "55px 0" },
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
    backgroundColor: colors.subtle,
    textAlign: "left",
  },
  downloadRow: {
    display: { default: "flex", [media.phone]: "block" },
    justifyContent: "space-between",
    alignItems: "center",
    gap: 45,
  },
  downloadHeading: { marginBlockEnd: 25 },
  requirements: { lineHeight: 1.7 },
});
