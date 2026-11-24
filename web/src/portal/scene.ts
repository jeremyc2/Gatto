import catUrl from "../../assets/images/portal-cat-thrust.webp";
import turnUrl from "../../assets/images/portal-cat-turn.webp";
import paperUrl from "../../assets/images/portal-watercolor-paper.webp";
import { isDarkTheme } from "../theme";
import { drawFissure, drawStone, fissureProfile } from "./fissure";
import { drawCatBehind, drawCatInFront, flightPose, projectReturnPose, returnPose } from "./flight";
import { ease, mix, type PortalFrame } from "./geometry";
import { PortalScenery } from "./scenery";
import { createSparkGlow, drawSparks, portalWindow, sparkLight } from "./sparks";

const arrivalTime = 5.65;
const interactionTime = 3.35;
export const flyThroughDuration = 2.3;
const settledTime = 2.12;

export interface PortalScene {
  arrivalTime: number;
  interactionTime: number;
  resize(width: number, height: number, fullHeight: number, linkWidth: number): void;
  paint(time: number): void;
  sceneryTime(): number;
  flyThrough(overlay: HTMLCanvasElement, time: number, start: number, elapsed: number): void;
}

async function loadImage(src: string): Promise<HTMLImageElement> {
  const image = new Image();
  image.src = src;
  await image.decode();
  return image;
}

function layer(width: number, height: number): CanvasRenderingContext2D {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Portal layer could not create a drawing context");
  return context;
}

export async function createPortalScene(canvas: HTMLCanvasElement): Promise<PortalScene> {
  const [cat, paper, turn] = await Promise.all([
    loadImage(catUrl),
    loadImage(paperUrl),
    loadImage(turnUrl).catch(() => undefined),
  ]);
  const ctx = canvas.getContext("2d", { alpha: false });
  if (!ctx) throw new Error("Portal canvas could not create a drawing context");
  const scenery = new PortalScenery();
  const glow = createSparkGlow();

  let width = 0;
  let height = 0;
  let fullHeight = 0;
  let radius = 0;
  let ratioX = 1;
  let ratioY = 1;
  let cut: ReturnType<typeof fissureProfile> = [];
  let colors = { upper: "", lower: "", dark: false };
  let stone: CanvasRenderingContext2D | undefined;
  let settled: CanvasRenderingContext2D | undefined;

  function makeStone(frame: PortalFrame) {
    const base = layer(canvas.width, canvas.height);
    base.setTransform(ratioX, 0, 0, ratioY, 0, 0);
    const pattern = base.createPattern(paper, "repeat");
    if (pattern) drawStone(base, frame, pattern, colors.dark);
    base.fillStyle = colors.lower;
    base.fillRect(0, height, width, fullHeight - height);
    return base;
  }

  function resize(w: number, h: number, full: number, linkWidth: number) {
    if (w <= 0 || h <= 0) return;
    width = w;
    height = h;
    fullHeight = full;
    radius = Math.min(linkWidth * 0.497, h * 0.29);
    // Keep enough backing pixels for sharp sparks without filling an ultrawide
    // Retina buffer at native resolution on every frame.
    const ratio = Math.min(window.devicePixelRatio || 1, 2, Math.sqrt(1_650_000 / (w * full)));
    const pixelsWide = Math.max(1, Math.round(w * ratio));
    const pixelsHigh = Math.max(1, Math.round(full * ratio));
    if (canvas.width !== pixelsWide) canvas.width = pixelsWide;
    if (canvas.height !== pixelsHigh) canvas.height = pixelsHigh;
    ratioX = pixelsWide / w;
    ratioY = pixelsHigh / full;
    cut = fissureProfile(w, h, radius);
    stone = undefined;
    settled = undefined;
    colors = {
      upper: getComputedStyle(canvas).color,
      lower: getComputedStyle(canvas.parentElement ?? canvas).backgroundColor,
      dark: isDarkTheme(matchMedia("(prefers-color-scheme: dark)").matches),
    };
  }

  const frameAt = (time: number): PortalFrame => ({
    width,
    height,
    x: width / 2,
    y: height / 2,
    radius,
    time,
  });

  function settledStone(frame: PortalFrame): CanvasRenderingContext2D {
    if (!settled) {
      settled = stone ?? makeStone(frame);
      drawFissure(settled, { ...frame, time: settledTime }, cut, colors, sparkLight(settledTime));
      stone = undefined;
    }
    return settled;
  }

  const paint = (time: number, departure?: { start: number; elapsed: number }) => {
    if (!width || !height || !radius) return;
    const frame = frameAt(time);
    const pose = departure
      ? returnPose(frame, departure.start, departure.elapsed)
      : flightPose(frame);
    const destination = () => {
      drawDestination(ctx, frame, () => {
        scenery.draw(ctx, time, radius, frame.x, frame.y);
        ctx.translate(frame.x, frame.y);
        drawCatBehind(ctx, cat, pose, turn);
      });
    };
    const repaint = settled !== undefined;
    ctx.setTransform(ratioX, 0, 0, ratioY, 0, 0);
    ctx.save();
    if (time < settledTime) {
      stone ??= makeStone(frame);
      ctx.drawImage(stone.canvas, 0, 0, width, fullHeight);
      drawFissure(ctx, frame, cut, colors, sparkLight(time), destination);
    } else {
      const surface = settledStone(frame);
      // Restore only the pixels that can be touched by the moving effects
      // once the fissure settles. Align the restore to backing-store pixels.
      const left = Math.max(0, Math.floor((frame.x - radius * 3.6) * ratioX));
      const right = Math.min(canvas.width, Math.ceil((frame.x + radius * 3.6) * ratioX));
      if (repaint) {
        ctx.beginPath();
        ctx.rect(left / ratioX, 0, (right - left) / ratioX, fullHeight);
        ctx.clip();
        ctx.drawImage(
          surface.canvas,
          left,
          0,
          right - left,
          canvas.height,
          left / ratioX,
          0,
          (right - left) / ratioX,
          fullHeight,
        );
      } else {
        ctx.drawImage(surface.canvas, 0, 0, width, fullHeight);
      }
      destination();
    }
    drawSparks(ctx, frame, glow, fullHeight);
    ctx.translate(frame.x, frame.y);
    drawCatInFront(ctx, cat, pose, turn);
    ctx.restore();
  };
  function flyThrough(overlay: HTMLCanvasElement, time: number, start: number, elapsed: number) {
    const approach = ease(0.62, flyThroughDuration, elapsed);
    if (approach === 0 || !radius) {
      paint(time, { start, elapsed });
      return;
    }
    const camera = overlay.getContext("2d");
    const viewWidth = overlay.clientWidth;
    const viewHeight = overlay.clientHeight;
    if (!camera || !viewWidth || !viewHeight) return;
    const ratio = Math.min(
      devicePixelRatio || 1,
      2,
      Math.sqrt(2_500_000 / (viewWidth * viewHeight)),
    );
    const pixelsWide = Math.round(viewWidth * ratio);
    const pixelsHigh = Math.round(viewHeight * ratio);
    if (overlay.width !== pixelsWide) overlay.width = pixelsWide;
    if (overlay.height !== pixelsHigh) overlay.height = pixelsHigh;

    const rect = canvas.getBoundingClientRect();
    const scale = Math.exp(
      Math.log(Math.max(1, Math.hypot(viewWidth, viewHeight) / (radius * 1.72))) * approach,
    );
    const x = mix(rect.left + width / 2, viewWidth / 2, approach);
    const y = mix(rect.top + height / 2, viewHeight / 2, approach);
    const frame = frameAt(time);
    const pose = projectReturnPose(
      returnPose(frame, start, elapsed),
      { x, y, scale, width: viewWidth, hasTurn: turn !== undefined },
      elapsed,
    );

    camera.setTransform(pixelsWide / viewWidth, 0, 0, pixelsHigh / viewHeight, 0, 0);
    camera.clearRect(0, 0, viewWidth, viewHeight);
    if (approach < 1) {
      const surface = settledStone(frame);
      camera.save();
      camera.translate(x, y);
      camera.scale(scale, scale);
      camera.translate(-frame.x, -frame.y);
      camera.imageSmoothingEnabled = false;
      camera.drawImage(surface.canvas, 0, 0, width, fullHeight);
      camera.restore();

      // Keep the destination at its own scale while the projected opening
      // gets larger. The extra area is the same harbor coming into view.
      const rim = new Path2D();
      rim.addPath(portalWindow(frame), new DOMMatrix().translate(x, y).scale(scale));
      camera.save();
      camera.clip(rim);
      camera.imageSmoothingEnabled = true;
      scenery.draw(camera, time, radius, x, y, approach, radius * scale);
      if (pose) drawCatBehind(camera, cat, pose, turn);
      camera.restore();

      const projected = { ...frame, x, y, radius: radius * scale, height: viewHeight };
      drawSparks(camera, projected, glow, viewHeight);
      if (pose) drawCatInFront(camera, cat, pose, turn);
    } else {
      scenery.draw(camera, time, radius, x, y, 1);
    }
    if (overlay.style.visibility !== "visible") {
      overlay.style.visibility = "visible";
      document.documentElement.dataset.portalFlight = "";
    }
  }

  return {
    arrivalTime,
    interactionTime,
    resize,
    paint,
    flyThrough,
    sceneryTime: () => scenery.time,
  };
}

function drawDestination(ctx: CanvasRenderingContext2D, frame: PortalFrame, draw: () => void) {
  const reveal = ease(1.15, 1.78, frame.time);
  if (reveal === 0) return;
  const rim = new Path2D();
  rim.addPath(portalWindow(frame), new DOMMatrix().translate(frame.x, frame.y));
  ctx.save();
  ctx.clip(rim);
  ctx.globalAlpha = reveal;
  draw();
  ctx.restore();
}
