import { createSimulation } from "./simulation";
import { bindControls } from "./input/browser";
import { createRenderer } from "./rendering/phaser";
import type { Command, GameView } from "./simulation/contracts";

export type GameRuntime = {
  command(command: Command): void;
  steer(x: number, y: number): void;
  boost(active: boolean): void;
  destroy(): void;
};
export const freshSeed = () => crypto.getRandomValues(new Uint32Array(1))[0] ?? Date.now();

/** Owns browser lifetime and scheduling. The renderer receives only read models. */
export async function mountGame(
  host: HTMLElement,
  updateHud: (view: GameView) => void,
  signal: AbortSignal,
): Promise<GameRuntime | undefined> {
  const simulation = createSimulation(freshSeed());
  const renderer = await createRenderer(host);
  if (signal.aborted) {
    renderer.destroy();
    return undefined;
  }
  let frame = 0,
    previous = performance.now(),
    lastHud = 0,
    disposed = false;
  let current = simulation.read(),
    oldPhase = current.phase;
  function schedule() {
    if (!frame && !document.hidden && !disposed) frame = requestAnimationFrame(tick);
  }
  function refresh() {
    current = simulation.read();
    renderer.draw(current);
    updateHud(current);
  }
  function command(command: Command) {
    if (disposed) return;
    simulation.send(command);
    if (
      command.type === "move" ||
      command.type === "thrust" ||
      command.type === "walk-to" ||
      command.type === "interact"
    ) {
      schedule();
      return;
    }
    current = simulation.read();
    if (current.phase !== oldPhase) {
      oldPhase = current.phase;
      controls.clear();
      if (current.phase === "playing") host.focus({ preventScroll: true });
    }
    renderer.draw(current);
    updateHud(current);
    schedule();
  }
  const controls = bindControls(
    host,
    command,
    () => current,
    (x, y) => renderer.pick(x, y),
  );
  function tick(now: number) {
    frame = 0;
    simulation.step((now - previous) / 1000);
    previous = now;
    current = simulation.read();
    renderer.draw(current);
    if (now - lastHud > 100 || current.phase !== oldPhase) {
      lastHud = now;
      oldPhase = current.phase;
      updateHud(current);
      if (current.phase !== "playing") controls.clear();
    }
    if (current.phase === "playing") schedule();
  }
  const visibility = () => {
    if (document.hidden) {
      command({ type: "pause" });
      controls.clear();
      cancelAnimationFrame(frame);
      frame = 0;
      renderer.sleep(true);
    } else {
      previous = performance.now();
      renderer.sleep(false);
      refresh();
    }
  };
  const blur = () => {
    command({ type: "pause" });
    controls.clear();
  };
  document.addEventListener("visibilitychange", visibility);
  window.addEventListener("blur", blur);
  refresh();
  schedule();
  host.focus({ preventScroll: true });
  return {
    command,
    steer: (x, y) => controls.steer(x, y),
    boost: (active) => controls.boost(active),
    destroy() {
      disposed = true;
      cancelAnimationFrame(frame);
      controls.destroy();
      renderer.destroy();
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("blur", blur);
    },
  };
}
