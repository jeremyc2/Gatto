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
          <Link href="#top" label="Gatto home" variant="graphic">
            <Brand dot />
          </Link>
          <nav {...stylex.attrs(styles.nav)} aria-label="Main navigation">
            <Link
              xstyle={styles.navLink}
              href="https://github.com/jeremyc2/Gatto/blob/main/docs/WALKTHROUGH.md"
            >
              Walkthrough
            </Link>
            <Link href="https://github.com/jeremyc2/Gatto" label="GitHub" variant="icon">
              {/* GitHub's official mark from primer/octicons (MIT). */}
              <svg
                width="24"
                height="24"
                viewBox="0 0 16 16"
                fill="currentColor"
                aria-hidden="true"
              >
                <path d="M6.766 11.328c-2.063-.25-3.516-1.734-3.516-3.656 0-.781.281-1.625.75-2.188-.203-.515-.172-1.609.063-2.062.625-.078 1.468.25 1.968.703.594-.187 1.219-.281 1.985-.281.765 0 1.39.094 1.953.265.484-.437 1.344-.765 1.969-.687.218.422.25 1.515.046 2.047.5.593.766 1.39.766 2.203 0 1.922-1.453 3.375-3.547 3.64.531.344.89 1.094.89 1.954v1.625c0 .468.391.734.86.547C13.781 14.359 16 11.53 16 8.03 16 3.61 12.406 0 7.984 0 3.563 0 0 3.61 0 8.031a7.88 7.88 0 0 0 5.172 7.422c.422.156.828-.125.828-.547v-1.25c-.219.094-.5.156-.75.156-1.031 0-1.64-.562-2.078-1.609-.172-.422-.36-.672-.719-.719-.187-.015-.25-.093-.25-.187 0-.188.313-.328.625-.328.453 0 .844.281 1.25.86.313.452.64.655 1.031.655s.641-.14 1-.5c.266-.265.47-.5.657-.656" />
              </svg>
            </Link>
          </nav>
        </Container>
      </header>
    </>
  );
}
const styles = stylex.create({
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
    display: "inline-flex",
    minHeight: 44,
    alignItems: "center",
  },
});
