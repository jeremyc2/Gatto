import * as stylex from "@stylexjs/stylex";
import { colors, media } from "../design-system/tokens.stylex";
import cat from "../../assets/images/download-rocket-cat-front.webp";
import portal from "../../assets/images/download-portal.webp";
import paper from "../../assets/images/portal-watercolor-paper.webp";
import { SiteFooter } from "./SiteFooter";
import "./portal-entry.css";

export function PortalEnding(props: { onActivate: () => void }) {
  return (
    <div
      data-portal-finale=""
      {...stylex.attrs(styles.setting)}
      style={{ "--portal-paper": `url(${paper})` }}
    >
      <PortalCat onActivate={props.onActivate} />
      <SiteFooter />
      <div data-portal-runway="" aria-hidden="true" />
    </div>
  );
}

export function PortalCat(props: { onActivate: () => void }) {
  return (
    <section
      {...stylex.attrs(styles.root)}
      data-portal-section=""
      aria-label="A visitor from the Gatto multiverse"
    >
      <div {...stylex.attrs(styles.content)} data-portal-content="">
        <button
          {...stylex.attrs(styles.scene)}
          id="download-portal"
          type="button"
          onClick={props.onActivate}
          aria-label="Explore the Gattoverse"
        >
          <span data-portal-void-entry="" {...stylex.attrs(styles.layers)}>
            <span {...stylex.attrs(styles.aperture)} />
          </span>
          <span data-portal-entry="" {...stylex.attrs(styles.layers)}>
            <svg {...stylex.attrs(styles.ringFrame)} viewBox="0 0 1000 1000" aria-hidden="true">
              <defs>
                <filter id="portal-brush-edge" x="-25%" y="-25%" width="150%" height="150%">
                  <feTurbulence
                    type="fractalNoise"
                    baseFrequency="0.035"
                    numOctaves="2"
                    seed="8"
                    result="grain"
                  />
                  <feDisplacementMap
                    in="SourceGraphic"
                    in2="grain"
                    scale="24"
                    xChannelSelector="R"
                    yChannelSelector="G"
                  />
                  <feGaussianBlur stdDeviation="18" />
                </filter>
                <mask
                  id="portal-brush-mask"
                  maskUnits="userSpaceOnUse"
                  x="0"
                  y="0"
                  width="1000"
                  height="1000"
                >
                  <circle
                    data-portal-trace=""
                    cx="500"
                    cy="500"
                    r="400"
                    pathLength="1"
                    fill="none"
                    stroke="white"
                    stroke-width="320"
                    stroke-linecap="round"
                    stroke-dasharray="1 1"
                    stroke-dashoffset="0"
                    transform="rotate(-90 500 500)"
                    filter="url(#portal-brush-edge)"
                  />
                </mask>
              </defs>
              <g mask="url(#portal-brush-mask)">
                <image {...stylex.attrs(styles.ring)} href={portal} width="1000" height="1000" />
              </g>
            </svg>
          </span>
          <span data-portal-flight="" {...stylex.attrs(styles.layers)}>
            <img
              {...stylex.attrs(styles.cat)}
              src={cat}
              width="1254"
              height="1254"
              alt=""
              loading="lazy"
            />
          </span>
        </button>
      </div>
    </section>
  );
}

const spin = stylex.keyframes({ to: { transform: "rotate(360deg)" } });
const float = stylex.keyframes({
  "0%, 100%": { translate: "0 1px" },
  "50%": { translate: "0 -1px" },
});
const styles = stylex.create({
  setting: {
    backgroundColor: colors.portalPaper,
    backgroundImage: "var(--portal-paper)",
    backgroundRepeat: "repeat",
    backgroundSize: "512px 512px",
  },
  root: { overflow: "clip" },
  content: {
    boxSizing: "border-box",
    display: "flex",
    justifyContent: "center",
    paddingBlock: { default: 48, [media.phone]: 36 },
    translate: { default: "0 24px", [media.phone]: "0 20px" },
  },
  scene: {
    position: "relative",
    width: "min(340px, calc(100% - 36px))",
    aspectRatio: "1",
    overflow: "clip",
    padding: 0,
    borderWidth: 0,
    borderRadius: "50%",
    backgroundColor: "transparent",
    cursor: "pointer",
    touchAction: "manipulation",
    outline: { default: null, ":focus-visible": `3px solid ${colors.focus}` },
    outlineOffset: { default: null, ":focus-visible": 3 },
  },
  layers: { position: "absolute", inset: 0 },
  aperture: {
    position: "absolute",
    inset: "12%",
    backgroundImage: `radial-gradient(circle closest-side in oklab, ${colors.portalVoid} 0% 70%, ${colors.portalVoidDepth} 82%, ${colors.portalVoidHaze} 92%, transparent 100%)`,
  },
  ringFrame: {
    position: "absolute",
    inset: 0,
    width: "100%",
    height: "100%",
  },
  ring: {
    transformOrigin: "center",
    transformBox: "view-box",
    animationName: { default: spin, [media.reducedMotion]: "none" },
    animationDuration: "240s",
    animationTimingFunction: "linear",
    animationIterationCount: "infinite",
  },
  cat: {
    position: "absolute",
    width: "42%",
    height: "42%",
    insetInlineStart: "29%",
    insetBlockStart: "29%",
    animationName: { default: float, [media.reducedMotion]: "none" },
    animationDuration: "28s",
    animationTimingFunction: "ease-in-out",
    animationIterationCount: "infinite",
  },
});
