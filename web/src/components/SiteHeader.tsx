import { colors, layout, media, space } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Link } from "../design-system/Link";
import { Brand } from "./Brand";
export function SiteHeader() {
  return (
    <>
      <Link xstyle={styles.skip} href="#details">
        Skip the illustrated story
      </Link>
      <header {...stylex.attrs(styles.header)}>
        <Container xstyle={styles.headerInner}>
          <Link href="#top" label="Gatto home" xstyle={styles.home}>
            <Brand dot />
          </Link>
          <nav {...stylex.attrs(styles.nav)} aria-label="Main navigation">
            <Link
              xstyle={styles.navLink}
              href="https://github.com/jeremyc2/Gatto/blob/main/docs/WALKTHROUGH.md"
            >
              Walkthrough ↗
            </Link>
          </nav>
        </Container>
      </header>
    </>
  );
}
const styles = stylex.create({
  home: { textDecoration: "none" },
  skip: {
    boxSizing: "border-box",
    position: "fixed",
    top: { default: -100, ":focus": 15 },
    left: 20,
    zIndex: 100,
    backgroundColor: colors.accent,
    color: colors.onAccent,
    paddingBlock: space.md,
    paddingInline: 20,
  },
  header: {
    boxSizing: "border-box",
    height: layout.headerHeight,
    position: "sticky",
    top: 0,
    zIndex: 20,
    backgroundColor: colors.canvas,
    borderBottomWidth: 1,
    borderBottomStyle: "solid",
    borderBottomColor: colors.border,
  },
  headerInner: {
    height: "100%",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 25,
  },
  nav: {
    display: "flex",
    gap: { default: space.xl, [media.tablet]: 18, [media.phone]: 10 },
    alignItems: "center",
  },
  navLink: {
    fontSize: { default: 15, [media.phone]: 14 },
    textDecoration: { default: "none", ":hover": "underline" },
    display: "inline-flex",
    minHeight: 44,
    alignItems: "center",
  },
});
