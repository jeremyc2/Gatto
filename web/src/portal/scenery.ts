import { HarborMenuRenderer, harborMenuFrame } from "../rooftops/harbor/flight";
import { defaultAim } from "../rooftops/harbor/projection";
import { clamp, mix } from "./geometry";

export function sceneryProjection(
  viewWidth: number,
  viewHeight: number,
  radius: number,
  x: number,
  y: number,
  approach: number,
  projectedRadius = radius,
) {
  const harbor = harborMenuFrame(viewWidth, viewHeight);
  const cropRadius = Math.min(harbor.width, harbor.height) * 0.31;
  const edge = projectedRadius * 1.05;
  const left = clamp(x - edge, 0, viewWidth);
  const top = clamp(y - edge, 0, viewHeight);
  const right = clamp(x + edge, 0, viewWidth);
  const bottom = clamp(y + edge, 0, viewHeight);
  // Frame every visible part of the aperture, even when it is taller than
  // the viewport mid-flight. The underlying harbor stays at native scale.
  const scale = Math.max(
    Math.min(1, radius / cropRadius) ** (1 - approach),
    (right - left) / viewWidth,
    (bottom - top) / viewHeight,
  );
  const focusX = mix(harbor.x + harbor.width * 0.49, viewWidth / 2, approach);
  const focusY = mix(harbor.y + harbor.height * 0.64, viewHeight / 2, approach);
  return {
    x: clamp(x - focusX * scale, right - viewWidth * scale, left),
    y: clamp(y - focusY * scale, bottom - viewHeight * scale, top),
    scale,
  };
}

export class PortalScenery {
  private readonly renderer = new HarborMenuRenderer();
  private readonly canvas = document.createElement("canvas");
  private readonly ctx = this.canvas.getContext("2d", { alpha: false });
  private readonly aim = defaultAim();
  private width = 0;
  private height = 0;
  private phase = -1;

  get time(): number {
    return Math.max(0, this.phase);
  }

  draw(
    ctx: CanvasRenderingContext2D,
    time: number,
    radius: number,
    x: number,
    y: number,
    approach = 0,
    projectedRadius = radius,
  ) {
    if (!this.ctx) return;
    const width = window.innerWidth;
    const height = window.innerHeight;
    const phase = Math.floor(Math.max(0, time) * 18) / 18;
    if (width !== this.width || height !== this.height) {
      const ratio = Math.min(devicePixelRatio || 1, 2, Math.sqrt(2_500_000 / (width * height)));
      this.canvas.width = Math.max(1, Math.round(width * ratio));
      this.canvas.height = Math.max(1, Math.round(height * ratio));
      this.ctx.setTransform(this.canvas.width / width, 0, 0, this.canvas.height / height, 0, 0);
      this.width = width;
      this.height = height;
      this.phase = -1;
    }
    if (this.phase !== phase) {
      this.renderer.renderMenu(this.ctx, width, height, phase, this.aim, false, 0);
      this.phase = phase;
    }
    const view = sceneryProjection(width, height, radius, x, y, approach, projectedRadius);
    ctx.drawImage(this.canvas, view.x, view.y, width * view.scale, height * view.scale);
  }
}
