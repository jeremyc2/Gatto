import { ease, mix, progress, type PortalFrame } from "./geometry";

export interface FlightPose {
  x: number;
  y: number;
  size: number;
  cel: number;
  crossing: number;
  visibility: number;
  turn: number;
}

const frameTime = 0.075;
// The cat banks from facing the viewer, through a three-quarter profile, to a
// rear view. Keep in sync with the bank cels in scripts/render-portal-cat.ts.
const bankFrames = 18;
const bankTime = 0.048;
const turnCues = Array.from({ length: bankFrames }, (_, i) => (i + 1) * bankTime);
const firstRearFrame = turnCues.length;
const rearFrames = 6;
const turnWidth = 320;
const foregroundDepth = 0.84;
const hoverDepth = 1.31;

interface ReturnPose extends FlightPose {
  depth: number;
}

function cruise(t: number): number {
  const ramp = 0.16;
  if (t < ramp) return (t * t) / (2 * ramp * (1 - ramp));
  if (t > 1 - ramp) return 1 - (1 - t) ** 2 / (2 * ramp * (1 - ramp));
  return (t - ramp / 2) / (1 - ramp);
}

export function flightPose(frame: PortalFrame): FlightPose {
  const { time, radius } = frame;
  const path = cruise(progress(1.88, 4.1, time));
  const hover = 0.004 * ease(4.1, 5.65, time) * Math.sin((time - 4.1) * 2.2);
  return {
    // Straight out of the middle of the portal toward the viewer.
    x: radius * mix(0, 0.04, path),
    y: radius * (mix(0.06, -0.08, path) + hover),
    size: radius * mix(0.18, 1.35, path),
    cel: Math.floor(time / frameTime),
    crossing: ease(0.44, 0.69, path),
    visibility: ease(1.88, 2.05, time),
    turn: -1,
  };
}

export function returnPose(frame: PortalFrame, launchTime: number, elapsed: number): ReturnPose {
  const start = flightPose({ ...frame, time: launchTime });
  const align = ease(0.2, 0.8, elapsed);
  const depth = mix(foregroundDepth, hoverDepth, cruise(progress(0.32, 1.7, elapsed)));
  const scale = foregroundDepth / depth;
  const firstRearAt = turnCues[firstRearFrame - 1]!;
  const bank = turnCues.findIndex((cue) => elapsed < cue);
  const rear =
    firstRearFrame +
    (Math.floor(Math.max(0, elapsed - firstRearAt + 0.009) / frameTime) % rearFrames);
  return {
    // Slightly right of centre, so the camera passes beside the cat.
    x: mix(start.x, frame.radius * 0.3, align) * scale,
    y: mix(start.y, -frame.radius * 0.12, align) * scale,
    size: start.size * scale,
    depth,
    cel: Math.floor(frame.time / frameTime),
    crossing: start.crossing * (1 - ease(0.9, 1.12, depth)),
    visibility: 1,
    turn: bank < 0 ? rear : bank,
  };
}

export function projectReturnPose(
  pose: ReturnPose,
  camera: { x: number; y: number; scale: number; width: number; hasTurn: boolean },
  elapsed: number,
): FlightPose | undefined {
  // Once the aperture has expanded, the camera closes the gap to the cat's
  // hover position along the viewing axis. The cat stays in the same lane.
  const dolly = mix(1 - 1 / camera.scale, hoverDepth, ease(1.63, 2.26, elapsed));
  const relativeDepth = 1 - dolly / pose.depth;
  if (relativeDepth <= 0) return;
  const size = pose.size / relativeDepth;
  const x = camera.x + pose.x / relativeDepth;
  // Distance from the cel centre to the drawing's left edge, as a fraction of
  // its height. The rear cels are narrow; the others include the exhaust.
  const left = camera.hasTurn && pose.turn >= firstRearFrame ? 0.14 : 0.48;
  if (x - left * size >= camera.width) return;
  return {
    ...pose,
    x,
    y: camera.y + pose.y / relativeDepth,
    size,
  };
}

// Nearest-neighbour drawing at a fractional, changing scale doubles uneven rows
// and columns of art pixels, which read as a seam sweeping through the cat.
// Instead, each cel is enlarged once by a whole number with nearest-neighbour
// sampling, then drawn with smoothing, which keeps pixels crisp and even.
const prescale = 3;
const cels = new Map<string, HTMLCanvasElement>();

function cel(image: HTMLImageElement, x: number, width: number, height: number) {
  const key = `${image.src}#${x}`;
  let canvas = cels.get(key);
  if (canvas) {
    cels.delete(key);
  } else {
    canvas = document.createElement("canvas");
    canvas.width = width * prescale;
    canvas.height = height * prescale;
    const context = canvas.getContext("2d");
    if (!context) return;
    context.imageSmoothingEnabled = false;
    context.drawImage(image, x, 0, width, height, 0, 0, canvas.width, canvas.height);
    if (cels.size >= 10) cels.delete(cels.keys().next().value!);
  }
  cels.set(key, canvas);
  return canvas;
}

function sprite(
  ctx: CanvasRenderingContext2D,
  cat: HTMLImageElement,
  pose: FlightPose,
  turn: HTMLImageElement | undefined,
) {
  const tile = cat.naturalHeight;
  const turning = turn !== undefined && pose.turn >= 0;
  const image = turning ? turn : cat;
  const width = turning ? turnWidth : tile;
  const sourceX = turning ? pose.turn * width : (pose.cel % (cat.naturalWidth / tile)) * tile;
  const drawWidth = pose.size * (width / tile);
  const enlarged = Math.abs(ctx.getTransform().a) * pose.size > tile;
  const source = enlarged ? cel(image, sourceX, width, tile) : undefined;
  ctx.save();
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.translate(pose.x, pose.y);
  if (source) {
    ctx.drawImage(source, -drawWidth / 2, -pose.size / 2, drawWidth, pose.size);
  } else {
    ctx.drawImage(
      image,
      sourceX,
      0,
      width,
      tile,
      -drawWidth / 2,
      -pose.size / 2,
      drawWidth,
      pose.size,
    );
  }
  ctx.restore();
}

function clipEmergingSprite(ctx: CanvasRenderingContext2D, pose: FlightPose) {
  const reach = pose.size * 2;
  const split = pose.size * (0.71 - pose.crossing * 1.42);
  const transform = ctx.getTransform();
  ctx.translate(pose.x, pose.y);
  // The head leads, so the part in front of the rim grows from the top down.
  ctx.rotate(-Math.PI / 2);
  ctx.beginPath();
  ctx.rect(split, -reach, reach, reach * 2);
  ctx.clip();
  ctx.setTransform(transform);
}

export function drawCatBehind(
  ctx: CanvasRenderingContext2D,
  cat: HTMLImageElement,
  pose: FlightPose,
  turn?: HTMLImageElement,
) {
  if (pose.visibility === 0 || pose.crossing === 1) return;
  ctx.save();
  ctx.globalAlpha = pose.visibility;
  // The aperture already clips this pass. A complete cel under the emerging
  // part keeps its antialiased clipping edge from cutting through solid fur.
  sprite(ctx, cat, pose, turn);
  ctx.restore();
}

export function drawCatInFront(
  ctx: CanvasRenderingContext2D,
  cat: HTMLImageElement,
  pose: FlightPose,
  turn?: HTMLImageElement,
) {
  if (pose.crossing === 0) return;
  ctx.save();
  if (pose.crossing < 1) clipEmergingSprite(ctx, pose);
  sprite(ctx, cat, pose, turn);
  ctx.restore();
}
