import type { Command, GameView, Point } from "../simulation/contracts";

type Pick = (x: number, y: number) => { point: Point; entity?: number };

/** Browser events become commands; neither the browser nor Phaser updates component stores. */
export function bindControls(
  host: HTMLElement,
  send: (command: Command) => void,
  view: () => GameView,
  pick: Pick,
) {
  const keys = new Set<string>();
  let stick = { x: 0, y: 0 },
    touchThrust = false;
  const move = () =>
    send({
      type: "move",
      x: Math.max(
        -1,
        Math.min(
          1,
          stick.x +
            Number(keys.has("d") || keys.has("arrowright")) -
            Number(keys.has("a") || keys.has("arrowleft")),
        ),
      ),
      y: Math.max(
        -1,
        Math.min(
          1,
          stick.y +
            Number(keys.has("s") || keys.has("arrowdown")) -
            Number(keys.has("w") || keys.has("arrowup")),
        ),
      ),
    });
  const thrust = () => send({ type: "thrust", active: touchThrust || keys.has(" ") });
  const onKeyDown = (event: KeyboardEvent) => {
    const key = event.key.toLowerCase();
    if (
      [
        "arrowleft",
        "arrowright",
        "arrowup",
        "arrowdown",
        "w",
        "a",
        "s",
        "d",
        " ",
        "e",
        "j",
        "escape",
        "p",
        "?",
      ].includes(key)
    )
      event.preventDefault();
    keys.add(key);
    if (!event.repeat) {
      if (key === "e") send({ type: "interact" });
      else if (key === "j") send({ type: "launch" });
      else if (key === "escape" || key === "p") send({ type: "pause" });
      else if (key === "?") send({ type: "help" });
    }
    move();
    thrust();
  };
  const onKeyUp = (event: KeyboardEvent) => {
    keys.delete(event.key.toLowerCase());
    move();
    thrust();
  };
  const onPointerDown = (event: PointerEvent) => {
    if (event.button !== 0 || view().phase !== "playing") return;
    host.focus({ preventScroll: true });
    host.setPointerCapture(event.pointerId);
    if (view().mode === "roof") {
      const target = pick(event.clientX, event.clientY);
      send({ type: "walk-to", ...target });
    } else {
      touchThrust = true;
      thrust();
    }
  };
  const onPointerUp = () => {
    touchThrust = false;
    thrust();
  };
  function clear() {
    keys.clear();
    stick = { x: 0, y: 0 };
    touchThrust = false;
    if (view().phase === "playing") {
      move();
      thrust();
    }
  }
  host.addEventListener("keydown", onKeyDown);
  host.addEventListener("keyup", onKeyUp);
  host.addEventListener("pointerdown", onPointerDown);
  host.addEventListener("pointerup", onPointerUp);
  host.addEventListener("pointercancel", onPointerUp);
  host.addEventListener("lostpointercapture", onPointerUp);
  host.addEventListener("blur", clear);
  return {
    steer(x: number, y: number) {
      stick = { x, y };
      move();
    },
    boost(active: boolean) {
      touchThrust = active;
      thrust();
    },
    clear,
    destroy() {
      host.removeEventListener("keydown", onKeyDown);
      host.removeEventListener("keyup", onKeyUp);
      host.removeEventListener("pointerdown", onPointerDown);
      host.removeEventListener("pointerup", onPointerUp);
      host.removeEventListener("pointercancel", onPointerUp);
      host.removeEventListener("lostpointercapture", onPointerUp);
      host.removeEventListener("blur", clear);
    },
  };
}
