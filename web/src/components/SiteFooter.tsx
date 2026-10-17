import { Link } from "../design-system/Link";
import { FontCredits } from "./FontCredits";
import { colors, media } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Brand } from "./Brand";
export function SiteFooter() {
  return (
    <footer {...stylex.attrs(styles.footer)}>
      <Container>
        <div {...stylex.attrs(styles.footerTop)}>
          <Brand small />
          <nav {...stylex.attrs(styles.footerNav)} aria-label="Project links">
            <Link href="https://github.com/jeremyc2/Gatto">GitHub</Link>
            <Link href="https://github.com/jeremyc2/Gatto/blob/main/SUPPORT.md">Support</Link>
            <Link href="https://github.com/jeremyc2/Gatto/blob/main/LICENSE">MIT license</Link>
          </nav>
        </div>
        <div {...stylex.attrs(styles.footerBottom)}>
          <p {...stylex.attrs(styles.copyright)}>Open source under the MIT license.</p>
          <FontCredits />
        </div>
      </Container>
    </footer>
  );
}
const styles = stylex.create({
  footer: {
    boxSizing: "border-box",
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
    paddingBlockStart: 32,
    paddingBlockEnd: 27,
  },
  footerTop: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: { default: "center", [media.phone]: "start" },
    gap: 25,
  },
  footerNav: {
    display: "flex",
    gap: { default: 23, [media.phone]: 13 },
    fontSize: 14,
    flexWrap: "wrap",
    justifyContent: { default: "normal", [media.phone]: "flex-end" },
    maxWidth: { default: "none", [media.phone]: 200 },
  },
  footerBottom: {
    display: { default: "flex", [media.phone]: "block" },
    justifyContent: "space-between",
    gap: 20,
    color: colors.muted,
    fontSize: 12,
    marginBlockStart: 25,
  },
  copyright: { margin: 0, marginBlockEnd: { default: 0, [media.phone]: 12 } },
});
