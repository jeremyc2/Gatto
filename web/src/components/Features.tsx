import { colors, media } from "../design-system/tokens.stylex";
import * as stylex from "@stylexjs/stylex";

import { Container } from "../design-system/Container";
import { Heading, Text } from "../design-system/Typography";
import { VisuallyHidden } from "../design-system/VisuallyHidden";
import { MediaFrame } from "../design-system/MediaFrame";
import batchImage from "../../assets/images/feature-batch-1240.webp";
import batchSmall from "../../assets/images/feature-batch-620.webp";
import annotateImage from "../../assets/images/feature-annotate-1240.webp";
import annotateSmall from "../../assets/images/feature-annotate-620.webp";
import compareImage from "../../assets/images/feature-compare-1240.webp";
import compareSmall from "../../assets/images/feature-compare-620.webp";

const features = [
  {
    title: "Bring a batch.",
    description: "Add several files at once.",
    image: batchImage,
    small: batchSmall,
    width: 1240,
    height: 1520,
    alt: "Gatto with an uploaded image, a video sending, and a queued image in the same batch.",
    crop: true,
  },
  {
    title: "Make your point.",
    description: "Annotate before uploading.",
    image: annotateImage,
    small: annotateSmall,
    width: 1240,
    height: 943,
    alt: "Gatto’s image editor with a note and arrow pointing to the active color in a screenshot.",
    crop: false,
  },
  {
    title: "Side by side.",
    description: "Compare two images.",
    image: compareImage,
    small: compareSmall,
    width: 1240,
    height: 866,
    alt: "Gatto’s comparison editor showing orange and black gardener cats in matching pixel-editor views.",
    crop: false,
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
              <MediaFrame xstyle={styles.screenshotFrame}>
                <img
                  {...stylex.attrs(styles.screenshot, feature.crop && styles.batchCrop)}
                  src={feature.image}
                  srcset={`${feature.small} 620w, ${feature.image} 1240w`}
                  sizes="(max-width: 760px) min(530px, calc(100vw - 36px)), (max-width: 1050px) calc((100vw - 146px) / 3), (min-width: 1500px) min(477px, calc((100vw - 230px) / 3)), min(437px, calc((100vw - 190px) / 3))"
                  width={feature.width}
                  height={feature.height}
                  alt={feature.alt}
                  loading="lazy"
                  decoding="async"
                />
              </MediaFrame>
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
    gap: { default: 45, [media.phone]: 40 },
    marginTop: 0,
  },
  feature: {
    boxSizing: "border-box",
    minWidth: 0,
    width: "100%",
    maxWidth: { default: "none", [media.phone]: 530 },
    marginInline: { default: 0, [media.phone]: "auto" },
  },
  screenshotFrame: {
    aspectRatio: "4 / 3",
    marginBlockEnd: 24,
    backgroundColor: colors.mediaSurface,
  },
  screenshot: {
    position: "absolute",
    inset: 0,
    display: "block",
    width: "100%",
    height: "100%",
    objectFit: "contain",
  },
  batchCrop: { objectFit: "cover", objectPosition: "50% 86%" },
  featureHeading: { marginBlockEnd: 14 },
  featureDescription: { maxWidth: { default: "29ch", [media.phone]: "38ch" }, marginBlockEnd: 13 },
});
