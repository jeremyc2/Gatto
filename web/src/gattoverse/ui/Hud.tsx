import * as stylex from "@stylexjs/stylex";
import { For } from "solid-js";
import type { Command, GameView } from "../simulation/contracts";
import { Button } from "../../design-system/Button";
import { colors, media } from "../../design-system/tokens.stylex";
import { Icon, type IconName } from "./Icon";
import coin from "../../../assets/images/gattoverse-coin.png";

export function Hud(props: { view: GameView | undefined; command: (command: Command) => void }) {
  const nearIcon = (): IconName =>
    props.view?.near?.kind === "case" ? "key" : props.view?.near?.kind === "cat" ? "talk" : "coin";
  return (
    <>
      <div {...stylex.attrs(styles.top)}>
        <div {...stylex.attrs(styles.left)}>
          <div {...stylex.attrs(styles.stamina)}>
            <Icon name="food" size={23} />
            <div
              role="progressbar"
              aria-label="Stamina"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.ceil(props.view?.stamina ?? 100)}
              {...stylex.attrs(styles.track)}
            >
              <div
                {...stylex.attrs(styles.fill)}
                style={{ transform: `scaleX(${(props.view?.stamina ?? 100) / 100})` }}
              />
            </div>
          </div>
          <div
            {...stylex.attrs(styles.pocket)}
            role="group"
            aria-label={`${props.view?.score ?? 0} coins scored, ${props.view?.wallet ?? 0} available, ${props.view?.keys ?? 0} keys`}
          >
            <img src={coin} width="30" height="30" alt="" />
            <span>{String(props.view?.score ?? 0).padStart(2, "0")}</span>
            <span {...stylex.attrs(styles.wallet)}>({props.view?.wallet ?? 0})</span>
            <span {...stylex.attrs(styles.divider)} />
            <Icon name="key" size={20} />
            <span>{props.view?.keys ?? 0}</span>
          </div>
        </div>
        <div {...stylex.attrs(styles.right)}>
          <div {...stylex.attrs(styles.actions)}>
            <Button
              label="Help"
              onClick={() => props.command({ type: "help" })}
              xstyle={styles.circle}
              disabled={!props.view || props.view.phase !== "playing"}
            >
              <Icon name="help" size={21} />
            </Button>
            <Button
              label="Pause"
              onClick={() => props.command({ type: "pause" })}
              xstyle={styles.circle}
              disabled={!props.view || props.view.phase !== "playing"}
            >
              <Icon name="pause" size={20} />
            </Button>
          </div>
          {props.view?.mode === "roof" && (
            <div {...stylex.attrs(styles.map)} aria-hidden="true">
              <svg viewBox="0 0 120 100" width="120" height="100">
                <path d="M8 8h104v84H8Z" fill="#d1c5d50a" stroke="#b1a6ba55" stroke-width="1" />
                <For
                  keyed={(e) => e.id}
                  each={props.view.entities.filter(
                    (e) =>
                      [
                        "key",
                        "food",
                        "case",
                        "vendor",
                        "watercolor",
                        "paper",
                        "velvet",
                        "rat",
                      ].includes(e.kind) && !e.used,
                  )}
                >
                  {(e) => (
                    <circle
                      cx={8 + (e().x / (props.view?.roof.width ?? 1)) * 104}
                      cy={8 + (e().y / (props.view?.roof.depth ?? 1)) * 84}
                      r={e().kind === "vendor" ? 3 : e().kind === "rat" ? 2.2 : 1.3}
                      fill={
                        e().kind === "rat"
                          ? "#e28a92"
                          : e().kind === "key" || e().kind === "vendor"
                            ? "#ffdba0"
                            : e().kind === "food"
                              ? "#87d0a5"
                              : e().kind === "case"
                                ? "#b5a9b5"
                                : "#bfa7ee"
                      }
                    />
                  )}
                </For>
                <circle
                  cx={8 + (props.view.player.x / props.view.roof.width) * 104}
                  cy={8 + (props.view.player.y / props.view.roof.depth) * 84}
                  r="3"
                  fill="#c4ffe0"
                  stroke="#304d43"
                  stroke-width="1.5"
                />
              </svg>
              <span {...stylex.attrs(styles.roofNumber)}>
                <Icon name="compass" size={13} />
                {String(props.view.roof.id + 1).padStart(2, "0")}
              </span>
            </div>
          )}
        </div>
      </div>
      <div {...stylex.attrs(styles.bottom)}>
        {props.view?.jetpack && props.view.mode === "roof" && (
          <Button
            label="Equip jetpack and take off"
            onClick={() => props.command({ type: "launch" })}
            xstyle={styles.pack}
          >
            <Icon name="pack" size={27} />
            <kbd {...stylex.attrs(styles.key)}>J</kbd>
          </Button>
        )}
        {props.view?.near && (
          <Button
            label={
              props.view.near.kind === "cat"
                ? "Talk to cat"
                : props.view.near.kind === "case"
                  ? "Open suitcase"
                  : "Buy jetpack for five coins"
            }
            onClick={() => props.command({ type: "interact" })}
            xstyle={styles.interact}
          >
            <Icon name={nearIcon()} size={25} />
            <kbd {...stylex.attrs(styles.key)}>E</kbd>
          </Button>
        )}
      </div>
    </>
  );
}

const styles = stylex.create({
  top: {
    position: "absolute",
    insetBlockStart: "max(20px, env(safe-area-inset-top))",
    insetInline: { default: 24, [media.phone]: 14 },
    display: "flex",
    alignItems: "start",
    justifyContent: "space-between",
    pointerEvents: "none",
    zIndex: 3,
    gap: 12,
  },
  left: { display: "grid", gap: 14 },
  stamina: {
    display: "flex",
    alignItems: "center",
    gap: 11,
    color: colors.gameMint,
    paddingBlock: 10,
    paddingInline: 13,
    backgroundColor: colors.gameGlass,
    borderRadius: 28,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    boxShadow: "0 4px 20px #0004",
  },
  track: {
    width: { default: "clamp(160px, 21vw, 310px)", [media.phone]: "clamp(95px, 33vw, 190px)" },
    height: 13,
    backgroundColor: "#080f19",
    borderRadius: 8,
    overflow: "hidden",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#c3ffe22b",
  },
  fill: {
    width: "100%",
    height: "100%",
    backgroundImage: "linear-gradient(0deg, #468b68, #a6e0aa)",
    transformOrigin: "left center",
    borderRadius: 6,
    transitionProperty: "transform",
    transitionDuration: { default: "120ms", [media.reducedMotion]: "0s" },
    boxShadow: "inset 0 1px 0 #f2ffdd77",
  },
  pocket: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    color: colors.gameGold,
    fontSize: 21,
    textShadow: "0 2px 5px #000",
    fontVariantNumeric: "tabular-nums",
    paddingInlineStart: 8,
  },
  wallet: { fontSize: 11, color: colors.gameMuted },
  divider: { height: 14, width: 1, backgroundColor: colors.gameGlassBorder, marginInline: 8 },
  right: { display: "grid", justifyItems: "end", gap: 16 },
  actions: { display: "flex", gap: 9, pointerEvents: "auto" },
  circle: {
    display: "grid",
    placeItems: "center",
    width: 44,
    minHeight: 44,
    borderRadius: "50%",
    color: colors.gameText,
    backgroundColor: colors.gameGlass,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
  },
  map: {
    backgroundColor: "#131a29b3",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    borderRadius: 9,
    boxShadow: "0 4px 20px #0003",
    display: { default: "grid", "@media (height <= 500px), (width <= 360px)": "none" },
  },
  roofNumber: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    color: colors.gameMuted,
    fontSize: 10,
    gap: 5,
    paddingBlockEnd: 6,
  },
  bottom: {
    position: "absolute",
    insetBlockEnd: "max(28px, env(safe-area-inset-bottom))",
    insetInline: 0,
    display: "flex",
    justifyContent: { default: "center", [media.phone]: "end" },
    alignItems: { default: "center", [media.phone]: "end" },
    flexDirection: { default: "row", [media.phone]: "column" },
    paddingInlineEnd: { default: 0, [media.phone]: 24 },
    gap: 12,
    pointerEvents: "none",
    zIndex: 3,
  },
  pack: {
    display: "flex",
    alignItems: "center",
    gap: 13,
    pointerEvents: "auto",
    paddingBlock: 10,
    paddingInline: 18,
    color: colors.gameMint,
    backgroundColor: colors.gameGlass,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    borderRadius: 30,
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
  },
  interact: {
    display: "flex",
    alignItems: "center",
    gap: 13,
    pointerEvents: "auto",
    paddingBlock: 10,
    paddingInline: 18,
    color: colors.gameGold,
    backgroundColor: colors.gameGlass,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#ffdba070",
    borderRadius: 30,
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
  },
  key: {
    fontFamily: "inherit",
    fontSize: 11,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: "#ffffff36",
    borderRadius: 4,
    paddingInline: 4,
    display: { default: "block", "@media (pointer: coarse)": "none" },
  },
});
