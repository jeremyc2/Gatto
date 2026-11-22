import { themeMeta } from "../theme";
import { savePortalArrival } from "../rooftops/harbor/portal-arrival";
import type { PortalScene } from "../portal/scene";

interface PortalElements {
  stage: HTMLElement;
  canvas: HTMLCanvasElement;
  overlay: HTMLCanvasElement;
  link: HTMLAnchorElement;
  toggle: HTMLButtonElement;
}

interface PortalCallbacks {
  onInteractive(value: boolean): void;
  onMotionControl(paused: boolean | undefined): void;
}

export function portalElements(
  stage: HTMLElement | undefined,
  canvas: HTMLCanvasElement | undefined,
  overlay: HTMLCanvasElement | undefined,
  link: HTMLAnchorElement | undefined,
  toggle: HTMLButtonElement | undefined,
): PortalElements | undefined {
  if (stage && canvas && overlay && link && toggle) return { stage, canvas, overlay, link, toggle };
}

export function observePortal(
  { stage, canvas, overlay, link, toggle }: PortalElements,
  callbacks: PortalCallbacks,
) {
  if (typeof IntersectionObserver !== "function" || typeof ResizeObserver !== "function") return;
  const motion = matchMedia("(prefers-reduced-motion: reduce)");
  const appearance = matchMedia("(prefers-color-scheme: dark)");
  let scene: PortalScene | undefined;
  let frame = 0;
  let time = 0;
  let painted = 0;
  let previous: number | undefined;
  let visible = false;
  let inView = false;
  let started = false;
  let paused = false;
  let disposed = false;
  let interactive = true;
  let flightDuration = 0;
  let departure: { start: number; elapsed: number; href: string; presented: boolean } | undefined;

  const stop = () => {
    cancelAnimationFrame(frame);
    frame = 0;
    previous = undefined;
  };

  const clearDeparture = () => {
    departure = undefined;
    overlay.style.visibility = "hidden";
    delete document.documentElement.dataset.portalFlight;
  };

  const paint = () => {
    if (!scene) return;
    if (departure) scene.flyThrough(overlay, time, departure.start, departure.elapsed);
    else scene.paint(time);
    painted = time;
    const open = time >= scene.interactionTime;
    if (open !== interactive) {
      interactive = open;
      callbacks.onInteractive(open);
    }
  };

  const tick = (now: number) => {
    frame = 0;
    if (!scene) return;
    const delta = Math.max(0, Math.min(0.08, (now - (previous ?? now)) / 1000));
    time += delta;
    previous = now;
    if (departure) {
      if (departure.presented) {
        savePortalArrival(scene.sceneryTime());
        location.assign(departure.href);
        return;
      }
      departure.elapsed = Math.min(flightDuration, departure.elapsed + delta);
      paint();
      departure.presented = departure.elapsed === flightDuration;
    } else if (time < scene.arrivalTime || time - painted >= 1 / 32) {
      paint();
    }
    frame = requestAnimationFrame(tick);
  };

  const sync = () => {
    if (!scene) return;
    if (motion.matches) {
      stop();
      if (departure) {
        const { href } = departure;
        clearDeparture();
        location.assign(href);
        return;
      }
      callbacks.onMotionControl(undefined);
      if (time < scene.arrivalTime) {
        time = scene.arrivalTime;
        paint();
      }
      return;
    }
    if (inView && !document.hidden) started = true;
    callbacks.onMotionControl(started ? paused : undefined);
    if (document.hidden || (!departure && (!visible || paused))) {
      stop();
    } else if (started && !frame) {
      frame = requestAnimationFrame(tick);
    }
  };

  const enter = (event: MouseEvent) => {
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      !scene ||
      !interactive ||
      motion.matches ||
      paused ||
      !visible
    )
      return;
    event.preventDefault();
    if (departure) return;
    departure = { start: time, elapsed: 0, href: link.href, presented: false };
    sync();
  };

  const cancelDeparture = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !departure || departure.presented) return;
    clearDeparture();
    paint();
    sync();
  };

  const restoreFromNavigation = () => {
    if (!departure?.presented) return;
    clearDeparture();
    paint();
    sync();
  };

  const toggleMotion = () => {
    if (departure) {
      clearDeparture();
      paint();
    }
    paused = !paused;
    sync();
  };

  const resize = () => {
    if (!scene) return;
    scene.resize(
      canvas.clientWidth,
      stage.clientHeight,
      canvas.clientHeight,
      link.getBoundingClientRect().width,
    );
    paint();
  };
  const size = new ResizeObserver(resize);
  const visibility = new IntersectionObserver(
    ([entry]) => {
      if (!entry) return;
      visible = entry.isIntersecting && entry.intersectionRatio > 0;
      inView = entry.intersectionRatio >= 0.998;
      sync();
    },
    { threshold: [0, 0.001, 0.998, 1] },
  );
  const preload = new IntersectionObserver(
    (entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      preload.disconnect();
      void import("../portal/scene")
        .then(({ createPortalScene, flyThroughDuration }) => {
          flightDuration = flyThroughDuration;
          return createPortalScene(canvas);
        })
        .then((loaded) => {
          if (disposed) return;
          scene = loaded;
          if (motion.matches) time = scene.arrivalTime;
          resize();
          size.observe(canvas);
          size.observe(stage);
          size.observe(link);
          sync();
        })
        .catch(() => {
          if (disposed) return;
          canvas.style.visibility = "hidden";
          callbacks.onInteractive(true);
        });
    },
    { rootMargin: "650px 0px" },
  );

  const prepare = requestAnimationFrame(() => {
    preload.observe(stage);
    visibility.observe(stage);
  });
  const theme = new MutationObserver(resize);
  const meta = themeMeta();
  if (meta) theme.observe(meta, { attributeFilter: ["content"] });
  toggle.addEventListener("click", toggleMotion);
  link.addEventListener("click", enter);
  window.addEventListener("keydown", cancelDeparture);
  window.addEventListener("pagereveal", restoreFromNavigation);
  window.addEventListener("pageshow", restoreFromNavigation);
  document.addEventListener("visibilitychange", sync);
  motion.addEventListener("change", sync);
  appearance.addEventListener("change", resize);

  return () => {
    disposed = true;
    cancelAnimationFrame(prepare);
    stop();
    size.disconnect();
    visibility.disconnect();
    theme.disconnect();
    clearDeparture();
    preload.disconnect();
    toggle.removeEventListener("click", toggleMotion);
    link.removeEventListener("click", enter);
    window.removeEventListener("keydown", cancelDeparture);
    window.removeEventListener("pagereveal", restoreFromNavigation);
    window.removeEventListener("pageshow", restoreFromNavigation);
    document.removeEventListener("visibilitychange", sync);
    motion.removeEventListener("change", sync);
    appearance.removeEventListener("change", resize);
  };
}
