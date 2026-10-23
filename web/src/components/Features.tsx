import { colors, media } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Heading, Text } from "../design-system/Typography";
import { VisuallyHidden } from "../design-system/VisuallyHidden";
const features = [
  { title: "Bring a batch.", description: "Add several files at once." },
  {
    title: "Make your point.",
    description: "Annotate before uploading.",
  },
  {
    title: "Side by side.",
    description: "Compare two images.",
  },
];
export function Features() {
  return (
    <section {...stylex.attrs(styles.details)} id="details">
      <Container>
        <VisuallyHidden as="h2">More ways to work</VisuallyHidden>
        <div {...stylex.attrs(styles.extraGrid)}>
          {features.map((feature) => (
            <article {...stylex.attrs(styles.feature)}>
              <Heading as="h3" size="subheading" xstyle={styles.featureHeading}>
                {feature.title}
              </Heading>
              <Text muted xstyle={styles.featureDescription}>
                {feature.description}
              </Text>
            </article>
          ))}
        </div>
      </Container>
    </section>
  );
}
const styles = stylex.create({
  details: {
    boxSizing: "border-box",
    padding: { default: "110px 0 115px", [media.phone]: "65px 0" },
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
  },
  extraGrid: {
    display: "grid",
    gridTemplateColumns: { default: "repeat(3, minmax(0, 1fr))", [media.phone]: "1fr" },
    gap: { default: 45, [media.phone]: 25 },
    marginTop: 0,
  },
  feature: {
    boxSizing: "border-box",
    paddingTop: 22,
    borderTopWidth: 1,
    borderTopStyle: "solid",
    borderTopColor: colors.border,
  },
  featureHeading: { marginBlockEnd: 14 },
  featureDescription: { maxWidth: { default: "29ch", [media.phone]: "38ch" }, marginBlockEnd: 13 },
});
