import * as stylex from "@stylexjs/stylex";
import { createEffect } from "solid-js";
import type { Command, GameView } from "../simulation/contracts";
import { Button } from "../../design-system/Button";
import { VisuallyHidden } from "../../design-system/VisuallyHidden";
import { colors, media, type } from "../../design-system/tokens.stylex";
import { Icon, type IconName } from "./Icon";

export function Menus(props: {
  view: GameView | undefined;
  best: number;
  command: (command: Command) => void;
  seed: () => number;
  focus: () => void;
}) {
  let dialog: HTMLDialogElement | undefined;
  const title = () =>
    props.view?.phase === "help"
      ? "How to play"
      : props.view?.phase === "dialogue"
        ? (props.view.dialogue?.name ?? "A visitor")
        : props.view?.phase === "over"
          ? "Game over"
          : "Paused";
  const dismiss = () => {
    if (props.view?.phase === "over") props.command({ type: "restart", seed: props.seed() });
    else props.command({ type: "resume" });
  };
  createEffect(
    () => props.view?.phase,
    (phase) => {
      if (phase && phase !== "playing") {
        if (!dialog?.open) dialog?.showModal();
      } else if (dialog?.open) dialog.close();
    },
  );
  const action = (name: IconName, label: string, command: Command, primary = false) => (
    <Button
      label={label}
      onClick={() => props.command(command)}
      xstyle={[styles.action, primary && styles.primary]}
    >
      <Icon name={name} size={primary ? 34 : 24} />
    </Button>
  );

  return (
    <dialog
      ref={(element) => {
        dialog = element;
      }}
      {...stylex.attrs(styles.dialog)}
      aria-labelledby="game-menu-heading"
      onCancel={(event) => {
        event.preventDefault();
        dismiss();
      }}
      onClose={() => {
        if (props.view?.phase === "playing") props.focus();
      }}
    >
      <VisuallyHidden as="h2" id="game-menu-heading">
        {title()}
      </VisuallyHidden>
      <Button label="Close menu" onClick={dismiss} xstyle={styles.close}>
        <Icon name="close" size={20} />
      </Button>
      {props.view?.phase === "help" ? (
        <>
          <div {...stylex.attrs(styles.helpTitle)}>
            <Icon name="help" size={31} />
            <h2 {...stylex.attrs(styles.helpHeading)}>A little help.</h2>
          </div>
          <div {...stylex.attrs(styles.helpRows)}>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="compass" />
              <p {...stylex.attrs(styles.helpCopy)}>
                <kbd>WASD</kbd> / arrow keys to walk. Or click a spot. On touch screens, use the
                thumbstick.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="key" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Find scattered keys. Press <kbd>E</kbd> beside luggage to open it. There are more
                suitcases than keys.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="coin" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Collect five coins for the rooftop machine. Spending coins keeps your total score.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="pack" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Equip your jetpack with <kbd>J</kbd>. Hold <kbd>Space</kbd>, ↑, or the screen to
                climb. Release to descend. Land from above.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="food" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Walking and flying consume stamina. Stop on roofs for food. The red edge means
                dinner can't wait.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="skull" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Avoid rats, owls, and unicorns. Unicorns chase briefly, then return home.{" "}
                <kbd>Esc</kbd> pauses.
              </p>
            </div>
            <div {...stylex.attrs(styles.helpRow)}>
              <Icon name="talk" />
              <p {...stylex.attrs(styles.helpCopy)}>
                Walk up to another cat and press <kbd>E</kbd>. Everyone here has a story.
              </p>
            </div>
          </div>
        </>
      ) : props.view?.phase === "dialogue" ? (
        <>
          <div {...stylex.attrs(styles.speechIcon)}>
            <Icon name="talk" size={35} />
          </div>
          <h2 {...stylex.attrs(styles.name)}>{props.view.dialogue?.name}</h2>
          <p {...stylex.attrs(styles.story)}>“{props.view.dialogue?.text}”</p>
          <div {...stylex.attrs(styles.dialogueAction)}>
            {action("arrow", "Close conversation", { type: "resume" }, true)}
          </div>
        </>
      ) : (
        <>
          <div {...stylex.attrs(styles.emblem)}>
            <Icon
              name={
                props.view?.phase === "over"
                  ? props.view.failure === "hunger"
                    ? "food"
                    : "skull"
                  : "pause"
              }
              size={44}
            />
          </div>
          <div {...stylex.attrs(styles.inventory)}>
            <span role="img" aria-label={`${props.view?.score ?? 0} coins scored`}>
              <Icon name="coin" size={25} /> {props.view?.score ?? 0}
            </span>
            <span role="img" aria-label={`${props.best} best score`}>
              <Icon name="trophy" size={23} /> {props.best}
            </span>
            <span role="img" aria-label={`${props.view?.keys ?? 0} keys`}>
              <Icon name="key" size={23} /> {props.view?.keys ?? 0}
            </span>
          </div>
          <div {...stylex.attrs(styles.actions)}>
            {props.view?.phase !== "over" &&
              action("play", "Resume game", { type: "resume" }, true)}
            {props.view?.phase === "paused" &&
              props.view.jetpack &&
              props.view.mode === "roof" &&
              action("pack", "Equip jetpack and take off", { type: "launch" })}
            {action(
              "restart",
              "Start a new run",
              { type: "restart", seed: props.seed() },
              props.view?.phase === "over",
            )}
            <a href="/" {...stylex.attrs(styles.home)} aria-label="Back to Gatto">
              <Icon name="home" size={24} />
            </a>
          </div>
        </>
      )}
    </dialog>
  );
}

const styles = stylex.create({
  dialog: {
    boxSizing: "border-box",
    position: "fixed",
    width: "min(450px, calc(100% - 32px))",
    maxHeight: "calc(100dvh - 48px)",
    padding: { default: 32, [media.phone]: 24 },
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    borderRadius: 22,
    backgroundColor: "#1b2335f5",
    color: colors.gameText,
    fontFamily: type.body,
    fontSize: 14,
    lineHeight: 1.65,
    boxShadow: "0 25px 100px #0007",
    "::backdrop": { backgroundColor: "#070e20a8" },
  },
  close: {
    position: "absolute",
    insetBlockStart: 9,
    insetInlineEnd: 9,
    width: 44,
    display: "grid",
    placeItems: "center",
    color: colors.gameMuted,
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
  },
  emblem: {
    display: "flex",
    justifyContent: "center",
    color: colors.gameGold,
    marginBlockStart: 24,
    marginBlockEnd: 30,
  },
  inventory: {
    display: "flex",
    justifyContent: "center",
    flexWrap: "wrap",
    gap: 22,
    color: colors.gameGold,
    fontSize: 25,
    marginBlockEnd: 25,
  },
  actions: {
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    gap: 12,
    marginBlockStart: 18,
    paddingBlockEnd: 10,
  },
  action: {
    display: "grid",
    placeItems: "center",
    width: 56,
    minHeight: 56,
    color: colors.gameText,
    backgroundColor: "#ffffff0b",
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    borderRadius: "50%",
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
  },
  primary: { color: colors.gameSky, backgroundColor: colors.gameMint, width: 72, minHeight: 72 },
  home: {
    display: "grid",
    placeItems: "center",
    width: 56,
    height: 56,
    color: colors.gameText,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
    borderRadius: "50%",
    outline: { default: null, ":focus-visible": `3px solid ${colors.gameMint}` },
    outlineOffset: 4,
  },
  helpTitle: {
    display: "flex",
    alignItems: "center",
    gap: 15,
    color: colors.gameMint,
    marginBlockEnd: 18,
  },
  helpHeading: { fontFamily: type.display, fontWeight: 400, fontSize: 28, marginBlock: 0 },
  helpCopy: { marginBlock: 0 },
  helpRows: { display: "grid", gap: 16 },
  helpRow: {
    display: "grid",
    gridTemplateColumns: "26px minmax(0, 1fr)",
    alignItems: "center",
    gap: 14,
    color: colors.gameText,
  },
  speechIcon: { color: colors.gameMint, marginBlockStart: 8 },
  name: {
    fontFamily: type.display,
    fontSize: 30,
    fontWeight: 400,
    marginBlockStart: 14,
    marginBlockEnd: 12,
  },
  story: { fontSize: 18, lineHeight: 1.8, color: colors.gameText },
  dialogueAction: { display: "flex", justifyContent: "end", marginBlockStart: 24 },
});
