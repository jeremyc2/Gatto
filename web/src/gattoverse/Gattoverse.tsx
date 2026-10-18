import * as stylex from "@stylexjs/stylex";
import { createEffect, createSignal } from "solid-js";
import type { Command, GameView } from "./simulation/contracts";
import type { GameRuntime } from "./runtime";
import { Button } from "../design-system/Button";
import { VisuallyHidden } from "../design-system/VisuallyHidden";
import { colors, media } from "../design-system/tokens.stylex";
import { Hud } from "./ui/Hud";
import { Menus } from "./ui/Menus";
import { TouchControls } from "./ui/TouchControls";
import { Icon } from "./ui/Icon";

export default function Gattoverse() {
  const [view, setView] = createSignal<GameView>();
  const [best, setBest] = createSignal(0);
  const [failed, setFailed] = createSignal(false);
  let host: HTMLDivElement | undefined;
  let runtime: GameRuntime | undefined;
  let bestScore = 0;
  const seed = () => crypto.getRandomValues(new Uint32Array(1))[0] ?? Date.now();
  const command = (command: Command) => runtime?.command(command);

  createEffect(
    () => undefined,
    () => {
      const abort = new AbortController();
      try {
        bestScore = Math.max(0, Number(localStorage.getItem("gattoverse.best.v1")) || 0);
        setBest(bestScore);
      } catch {
        /* Optional local record. */
      }
      void import("./runtime")
        .then(async ({ mountGame }) => {
          if (abort.signal.aborted || !host) return;
          runtime = await mountGame(
            host,
            (next) => {
              if (abort.signal.aborted) return;
              setView(next);
              if (next.score > bestScore) {
                bestScore = next.score;
                setBest(next.score);
                try {
                  localStorage.setItem("gattoverse.best.v1", String(next.score));
                } catch {
                  /* The run does not depend on storage. */
                }
              }
            },
            abort.signal,
          );
        })
        .catch(() => {
          if (!abort.signal.aborted) setFailed(true);
        });
      return () => {
        abort.abort();
        runtime?.destroy();
        runtime = undefined;
      };
    },
  );

  return (
    <main {...stylex.attrs(styles.game)} aria-label="Gattoverse">
      <VisuallyHidden as="h1">Gattoverse</VisuallyHidden>
      <div
        ref={(el) => {
          host = el;
        }}
        {...stylex.attrs(styles.host)}
        tabindex="0"
        role="group"
        aria-label="Game controls"
        aria-describedby="game-control-description"
      />
      <VisuallyHidden id="game-control-description">
        Use WASD or arrow keys to move, E to interact, J to equip a jetpack, Space to thrust, and
        Escape to pause. The help button contains full instructions.
      </VisuallyHidden>
      <div {...stylex.attrs(styles.vignette)} aria-hidden="true" />
      {view() && (view()?.stamina ?? 100) < 24 && view()?.phase === "playing" && (
        <div {...stylex.attrs(styles.hunger)} aria-hidden="true" />
      )}
      <Hud view={view()} command={command} />
      <TouchControls
        flight={view()?.mode === "flight"}
        active={view()?.phase === "playing"}
        steer={(x, y) => runtime?.steer(x, y)}
        boost={(active) => runtime?.boost(active)}
      />
      <Menus
        view={view()}
        best={best()}
        command={command}
        seed={seed}
        focus={() => host?.focus({ preventScroll: true })}
      />
      {!view() && (
        <div {...stylex.attrs(styles.loading)}>
          {failed() ? (
            <Button
              label="Reload game"
              onClick={() => window.location.reload()}
              xstyle={styles.reload}
            >
              <Icon name="restart" size={40} />
            </Button>
          ) : (
            <div {...stylex.attrs(styles.spinner)}>
              <Icon name="compass" size={38} />
            </div>
          )}
          <VisuallyHidden role="status" aria-live="polite">
            {failed()
              ? "The city couldn't load. Reload the game to try again."
              : "Loading Gattoverse"}
          </VisuallyHidden>
        </div>
      )}
      <noscript>
        <p {...stylex.attrs(styles.noScript)}>Enable JavaScript to play Gattoverse.</p>
      </noscript>
      <VisuallyHidden role="status" aria-live="polite">
        {view()?.phase === "over"
          ? `Game over. ${view()?.score ?? 0} gold coins.`
          : view()?.feedback
            ? `${view()?.feedback?.icon ?? ""} ${view()?.feedback?.amount ?? ""}`
            : ""}
      </VisuallyHidden>
    </main>
  );
}

const hungerPulse = stylex.keyframes({ "0%, 100%": { opacity: 0.18 }, "50%": { opacity: 0.7 } });
const breathe = stylex.keyframes({
  "0%, 100%": { opacity: 0.3, transform: "scale(0.9)" },
  "50%": { opacity: 1, transform: "scale(1)" },
});
const styles = stylex.create({
  game: {
    position: "fixed",
    inset: 0,
    height: "100dvh",
    overflow: "hidden",
    backgroundColor: colors.gameSky,
    color: colors.gameText,
    overscrollBehavior: "none",
  },
  host: {
    position: "absolute",
    inset: 0,
    touchAction: "none",
    outline: { default: null, ":focus-visible": `2px solid ${colors.gameMint}` },
    outlineOffset: -2,
  },
  vignette: {
    position: "absolute",
    inset: 0,
    pointerEvents: "none",
    backgroundImage: "radial-gradient(ellipse at 50% 48%, transparent 48%, #050b1b5c 100%)",
    zIndex: 1,
  },
  hunger: {
    position: "absolute",
    inset: 0,
    pointerEvents: "none",
    backgroundImage: "radial-gradient(ellipse at center, transparent 55%, #e82f3e99 100%)",
    boxShadow: "inset 0 0 35px #c42d3d66",
    zIndex: 2,
    animationName: { default: hungerPulse, [media.reducedMotion]: "none" },
    animationDuration: "1.4s",
    animationTimingFunction: "ease-in-out",
    animationIterationCount: "infinite",
    opacity: 0.45,
  },
  loading: {
    position: "absolute",
    inset: 0,
    display: "grid",
    placeItems: "center",
    backgroundColor: colors.gameSky,
    color: colors.gameMint,
    zIndex: 8,
  },
  spinner: {
    animationName: { default: breathe, [media.reducedMotion]: "none" },
    animationDuration: "1.8s",
    animationIterationCount: "infinite",
    animationTimingFunction: "ease-in-out",
  },
  reload: {
    width: 80,
    height: 80,
    borderRadius: "50%",
    display: "grid",
    placeItems: "center",
    color: colors.gameMint,
    borderWidth: 1,
    borderStyle: "solid",
    borderColor: colors.gameGlassBorder,
  },
  noScript: {
    position: "absolute",
    inset: "40% 10%",
    textAlign: "center",
    zIndex: 10,
    backgroundColor: colors.gamePanel,
    color: colors.gameText,
    padding: 24,
  },
});
