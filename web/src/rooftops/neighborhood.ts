import { createTileCanvas, type CachedTile } from "./rooms";
import {
  remEuclid,
  rgbCss,
  type CheckpointPalette,
  type NeighborhoodId,
  type PosterShape,
  type Rect4,
  type Roof,
} from "./types";

const FLOOR_HEIGHT = 82;
const WINDOW_TOP = 32;
const WINDOW_WIDTH = 66;
const ROOM_BOUNDS: Rect4 = [-6, -8, 78, 86];

interface NeighborhoodPalette {
  wall: number;
  trim: number;
  shadow: number;
  shutter: number;
}

export function neighborhoodForLevel(level: number): NeighborhoodId {
  if (level === 0) return "brownstone";
  if (level === 1) return "loft";
  return "limestone";
}

export function neighborhoodTitle(id: NeighborhoodId): string {
  switch (id) {
    case "brownstone":
      return "Brownstones";
    case "loft":
      return "SoHo";
    case "limestone":
      return "Midtown";
  }
}

function neighborhoodPalette(id: NeighborhoodId, seed: number, lit: boolean): NeighborhoodPalette {
  const idx = remEuclid(seed, 3);
  switch (id) {
    case "brownstone": {
      const walls = [0x60463f, 0x694b43, 0x5d4544] as const;
      const darkWalls = [0x433338, 0x49363a, 0x403239] as const;
      return {
        wall: (lit ? walls : darkWalls)[idx]!,
        trim: lit ? 0xa38772 : 0x7a625a,
        shadow: 0x32282f,
        shutter: 0x3d4c46,
      };
    }
    case "loft": {
      const walls = [0x405456, 0x48595a, 0x3b5053] as const;
      const darkWalls = [0x2b3b42, 0x303e44, 0x293940] as const;
      return {
        wall: (lit ? walls : darkWalls)[idx]!,
        trim: lit ? 0x8b9e96 : 0x617b78,
        shadow: 0x202e36,
        shutter: 0x41545a,
      };
    }
    case "limestone": {
      const walls = [0x757063, 0x6d6c64, 0x7b7468] as const;
      const darkWalls = [0x4f4d4c, 0x494b4d, 0x524e4d] as const;
      return {
        wall: (lit ? walls : darkWalls)[idx]!,
        trim: lit ? 0xb6a68e : 0x8b8276,
        shadow: 0x3b3940,
        shutter: 0x5f5b5b,
      };
    }
  }
}

export function checkpointPaletteFor(id: NeighborhoodId, seed: number): CheckpointPalette {
  const p = neighborhoodPalette(id, seed, true);
  return { wall: p.wall, coping: p.trim, frame: p.trim };
}

export function chimneyColorsFor(id: NeighborhoodId, seed: number, lit: boolean): [number, number] {
  const p = neighborhoodPalette(id, seed, lit);
  return [p.wall, p.trim];
}

function visibleFloorCount(top: number, height: number): number {
  return Math.max(0, Math.ceil((height - top) / FLOOR_HEIGHT));
}

export interface ApartmentLayout {
  readonly columns: number;
  readonly spacing: number;
}

export function createApartments(width: number): ApartmentLayout {
  const columns = Math.max(1, Math.floor((width - 20) / 74));
  return {
    columns,
    spacing: (width - 20) / columns,
  };
}

export function apartmentWindowX(layout: ApartmentLayout, column: number): number {
  return 10 + layout.spacing * (column + 0.5) - WINDOW_WIDTH * 0.5;
}

export function apartmentRows(roofY: number, height: number): [number, number][] {
  const top = roofY + WINDOW_TOP;
  const count = visibleFloorCount(top - 7, height);
  const rows: [number, number][] = [];
  for (let row = 0; row < count; row++) {
    rows.push([row, top + row * FLOOR_HEIGHT]);
  }
  return rows;
}

function surfaceShapes(
  id: NeighborhoodId,
  roof: Roof,
  seed: number,
  lit: boolean,
  height: number,
): PosterShape[] {
  const palette = neighborhoodPalette(id, seed, lit);
  const shapes: PosterShape[] = [];
  const add = (rect: Rect4, ink: number, opacity: number) => {
    shapes.push({ rect, ink, opacity, radius: 0 });
  };
  const { x, y, width } = roof;
  const left = x + 5;
  const right = x + width - 5;
  add([left, y + 10, right - left, Math.max(0, height - y)], palette.wall, 1);

  if (id === "brownstone") {
    add([x - 3, y, width + 6, 7], palette.trim, 1);
    add([x - 1, y + 7, width + 2, 5], palette.shadow, 1);
    add([x + 2, y + 12, width - 4, 5], palette.trim, 0.8);
    const brackets = Math.floor(width / 24);
    for (let b = 0; b < brackets; b++) {
      const bx = x + 12 + b * 24;
      add([bx, y + 16, 4, 8], palette.trim, 0.75);
      add([bx - 1, y + 23, 6, 2], palette.shadow, 0.8);
    }
    const floors = visibleFloorCount(y + 19, height);
    for (let row = 1; row < floors; row++) {
      const band = y + 19 + row * FLOOR_HEIGHT;
      add([left, band, right - left, 3], palette.trim, 0.38);
    }
  } else if (id === "loft") {
    add([x - 3, y, width + 6, 8], palette.trim, 1);
    add([x - 1, y + 8, width + 2, 7], palette.shadow, 1);
    add([x + 2, y + 15, width - 4, 5], palette.trim, 0.65);
    for (const edge of [left, right - 4]) {
      add([edge, y + 20, 4, Math.max(0, height - y - 20)], palette.trim, 0.6);
      add([edge + 1, y + 20, 1, Math.max(0, height - y - 20)], palette.shadow, 0.65);
    }
    const floors = visibleFloorCount(y + 20, height);
    const rivets = Math.floor((right - left) / 22);
    for (let row = 0; row < floors; row++) {
      const beam = y + 20 + row * FLOOR_HEIGHT;
      add([left, beam, right - left, 5], palette.shadow, 0.85);
      add([left, beam, right - left, 1], palette.trim, 0.8);
      for (let rivet = 0; rivet < rivets; rivet++) {
        add([left + 10 + rivet * 22, beam + 2, 1.5, 1.5], palette.trim, 0.7);
      }
    }
  } else {
    add([x - 3, y, width + 6, 6], palette.trim, 1);
    add([x - 1, y + 6, width + 2, 6], palette.shadow, 0.95);
    add([x + 2, y + 12, width - 4, 6], palette.trim, 0.8);
    for (const edge of [left + 1, right - 6]) {
      add([edge, y + 18, 5, Math.max(0, height - y - 18)], palette.trim, 0.45);
      for (const flute of [1, 3]) {
        add([edge + flute, y + 20, 0.6, Math.max(0, height - y - 20)], palette.shadow, 0.55);
      }
    }
    const floors = visibleFloorCount(y + 20, height);
    for (let row = 1; row < floors; row++) {
      const band = y + 20 + row * FLOOR_HEIGHT;
      add([left, band, right - left, 2], palette.trim, 0.5);
      add([left, band + 2, right - left, 1], palette.shadow, 0.5);
    }
  }
  add([x, y - 2, width, 2], palette.trim, 0.9);
  return shapes;
}

function surroundShapes(id: NeighborhoodId, seed: number, lit: boolean): PosterShape[] {
  const palette = neighborhoodPalette(id, seed, lit);
  const shapes: PosterShape[] = [];
  const add = (rect: Rect4, ink: number, opacity: number) => {
    shapes.push({ rect, ink, opacity, radius: 0 });
  };
  add([-3, 62, 72, 3], palette.trim, 1);
  add([-1, 65, 68, 1.5], palette.shadow, 0.75);
  if (id === "brownstone") {
    add([-2, -5, 70, 4], palette.trim, 0.9);
    add([29, -7, 8, 6], palette.trim, 1);
    if (lit) {
      for (const sx of [-4, 67]) {
        add([sx, 2, 3, 58], palette.shutter, 1);
        for (let row = 0; row < 10; row++) {
          add([sx, 5 + row * 5, 3, 0.7], palette.trim, 0.4);
        }
      }
    }
  } else if (id === "loft") {
    add([-3, -4, 72, 3], palette.trim, 1);
    for (const sx of [-4, 67]) {
      add([sx, 0, 3, 61], palette.trim, 0.9);
      add([sx + 1, 3, 1, 54], palette.shadow, 0.8);
      for (const sy of [1, 56]) {
        add([sx - 1, sy, 5, 3], palette.trim, 1);
      }
    }
  } else {
    add([-2, -5, 70, 4], palette.trim, 1);
    add([3, -7, 60, 2], palette.trim, 0.65);
    for (const sx of [-3, 67]) {
      add([sx, 0, 2, 60], palette.shadow, 0.8);
    }
    for (const [dx, dy, width] of [
      [23, 68, 20],
      [27, 71, 12],
      [31, 74, 4],
    ] as const) {
      add([dx, dy, width, 2], palette.trim, 0.55);
    }
  }
  return shapes;
}

function coveringShapes(id: NeighborhoodId, seed: number): PosterShape[] {
  const palette = neighborhoodPalette(id, seed, false);
  const shapes: PosterShape[] = [];
  const add = (rect: Rect4, ink: number, opacity: number) => {
    shapes.push({ rect, ink, opacity, radius: 0 });
  };
  add([2, 2, 62, 58], palette.shutter, 1);
  if (id === "brownstone") {
    for (const sx of [4, 35]) {
      add([sx, 5, 27, 50], palette.shadow, 0.7);
      for (let row = 0; row < 9; row++) {
        const sy = 7 + row * 5;
        add([sx + 2, sy, 23, 3], palette.shutter, 1);
        add([sx + 2, sy, 23, 0.7], palette.trim, 0.3);
      }
    }
    add([32, 2, 2, 58], palette.shadow, 0.8);
    for (const sx of [3, 60]) {
      for (const sy of [13, 45]) {
        add([sx, sy, 3, 4], palette.shadow, 1);
      }
    }
    add([30, 31, 6, 2], palette.trim, 0.8);
  } else if (id === "loft") {
    for (let col = 0; col < 10; col++) {
      const sx = 4 + col * 6;
      add([sx, 4, 1, 52], palette.trim, 0.35);
      add([sx + 3, 4, 1, 52], palette.shadow, 0.65);
    }
    for (const sy of [8, 49]) {
      add([3, sy, 60, 3], palette.shadow, 0.9);
      for (const sx of [8, 27, 39, 58]) {
        add([sx, sy + 1, 1.5, 1.5], palette.trim, 0.7);
      }
    }
    add([32, 2, 2, 58], palette.shadow, 1);
    add([28, 30, 10, 3], palette.trim, 0.7);
  } else {
    for (let row = 0; row < 10; row++) {
      const sy = 3 + row * 5.4;
      add([3, sy, 60, 1], palette.trim, 0.35);
      add([3, sy + 4, 60, 1], palette.shadow, 0.7);
    }
    for (const sx of [15, 50]) {
      add([sx, 3, 0.8, 53], palette.trim, 0.35);
    }
    add([3, 56, 60, 3], palette.shadow, 0.9);
  }
  return shapes;
}

function addFrame(shapes: PosterShape[], ink: number): void {
  for (const rect of [
    [0, 0, 66, 2],
    [0, 60, 66, 2],
    [0, 2, 2, 58],
    [64, 2, 2, 58],
  ] as const) {
    shapes.push({ rect: [rect[0], rect[1], rect[2], rect[3]], ink, opacity: 1, radius: 0 });
  }
}

function tileFromShapes(bounds: Rect4, shapes: readonly PosterShape[]): CachedTile {
  return createTileCanvas(bounds[0], bounds[1], bounds[2], bounds[3], (ctx) => {
    for (const shape of shapes) {
      const [x, y, w, h] = shape.rect;
      if (w <= 0 || h <= 0) continue;
      ctx.globalAlpha = shape.opacity;
      ctx.fillStyle = rgbCss(shape.ink);
      ctx.fillRect(x, y, w, h);
    }
    ctx.globalAlpha = 1;
  });
}

function masonryTile(id: NeighborhoodId, seed: number, lit: boolean): CachedTile {
  const palette = neighborhoodPalette(id, seed, lit);
  const [brick, course] =
    id === "brownstone" && remEuclid(seed, 3) === 1
      ? [26, 9]
      : id === "brownstone"
        ? [43, 18]
        : id === "loft"
          ? [30, 11]
          : [48, 22];
  const width = brick * 4;
  const height = course * 8;
  const shapes: PosterShape[] = [];
  const add = (rect: Rect4, ink: number, opacity: number) => {
    shapes.push({ rect, ink, opacity, radius: 0 });
  };
  for (let row = 0; row < 8; row++) {
    const y = row * course;
    add([0, y, width, 0.8], palette.shadow, 0.42);
    add([0, y + 0.8, width, 0.6], palette.trim, 0.1);
    const stagger = row % 2 === 0 ? 0 : brick * 0.5;
    for (let col = -1; col < 4; col++) {
      const x = col * brick + stagger;
      add([x, y, 0.8, course], palette.shadow, 0.35);
      if ((row + remEuclid(col, 4) + seed) % 4 === 0) {
        add([x + 1, y + 2, brick - 2, course - 3], palette.trim, 0.045);
      }
    }
  }
  return tileFromShapes([0, 0, width, height], shapes);
}

/**
 * Caches pre-rendered masonry tiles, window frames, closed shutter tiles,
 * and complete building walls into offscreen canvases for O(1) drawImage blits.
 */
export class FacadeArt {
  private cacheHeight = 0;
  private readonly walls = new Map<string, CachedTile>();
  private readonly materials = new Map<string, CachedTile>();
  private readonly surrounds = new Map<string, CachedTile>();
  private readonly closedRooms = new Map<string, CachedTile>();

  resize(height: number): void {
    const snapped = Math.ceil(height / FLOOR_HEIGHT) * FLOOR_HEIGHT;
    if (this.cacheHeight !== snapped) {
      this.walls.clear();
      this.cacheHeight = snapped;
    }
  }

  private masonry(id: NeighborhoodId, seed: number, lit: boolean): CachedTile {
    const key = `${id}:${remEuclid(seed, 3)}:${lit ? 1 : 0}`;
    let tile = this.materials.get(key);
    if (!tile) {
      tile = masonryTile(id, seed, lit);
      this.materials.set(key, tile);
    }
    return tile;
  }

  /** Pre-renders the entire building facade (base wall + tiled masonry + trim) once per building. */
  wallTile(id: NeighborhoodId, roof: Roof, seed: number, lit: boolean): CachedTile {
    const s = remEuclid(seed, 3);
    const key = `${id}:${s}:${roof.width}:${roof.y}:${lit ? 1 : 0}`;
    let tile = this.walls.get(key);
    if (!tile) {
      const h = Math.max(this.cacheHeight, roof.y + FLOOR_HEIGHT * 4);
      const shapes = surfaceShapes(id, { x: 0, y: roof.y, width: roof.width }, s, lit, h);
      const material = this.masonry(id, s, lit);
      const offsetX = -3;
      const offsetY = roof.y - 2;
      const totalW = roof.width + 6;
      const totalH = Math.max(10, h - offsetY);
      tile = createTileCanvas(offsetX, offsetY, totalW, totalH, (ctx) => {
        const base = shapes[0];
        if (base) {
          ctx.globalAlpha = base.opacity;
          ctx.fillStyle = rgbCss(base.ink);
          ctx.fillRect(base.rect[0], base.rect[1], base.rect[2], base.rect[3]);
        }
        const top = roof.y + 21;
        const innerW = roof.width - 10;
        const innerH = Math.max(0, h - top);
        if (innerW > 0 && innerH > 0) {
          ctx.save();
          ctx.beginPath();
          ctx.rect(5, top, innerW, innerH);
          ctx.clip();
          const rows = Math.ceil(innerH / material.height);
          const cols = Math.ceil(innerW / material.width);
          for (let r = 0; r < rows; r++) {
            for (let c = 0; c < cols; c++) {
              ctx.drawImage(
                material.canvas,
                5 + c * material.width,
                top + r * material.height,
                material.width,
                material.height,
              );
            }
          }
          ctx.restore();
        }
        for (let i = 1; i < shapes.length; i++) {
          const shape = shapes[i]!;
          ctx.globalAlpha = shape.opacity;
          ctx.fillStyle = rgbCss(shape.ink);
          ctx.fillRect(shape.rect[0], shape.rect[1], shape.rect[2], shape.rect[3]);
        }
        ctx.globalAlpha = 1;
      });
      this.walls.set(key, tile);
    }
    return tile;
  }

  openFrame(id: NeighborhoodId, seed: number): CachedTile {
    const s = remEuclid(seed, 3);
    const key = `${id}:${s}`;
    let tile = this.surrounds.get(key);
    if (!tile) {
      const color = neighborhoodPalette(id, s, true).trim;
      const shapes = surroundShapes(id, s, true);
      addFrame(shapes, color);
      tile = tileFromShapes(ROOM_BOUNDS, shapes);
      this.surrounds.set(key, tile);
    }
    return tile;
  }

  closedRoom(id: NeighborhoodId, seed: number): CachedTile {
    const s = remEuclid(seed, 3);
    const key = `${id}:${s}`;
    let tile = this.closedRooms.get(key);
    if (!tile) {
      const shapes = coveringShapes(id, s);
      shapes.push(...surroundShapes(id, s, false));
      addFrame(shapes, neighborhoodPalette(id, s, false).trim);
      tile = tileFromShapes(ROOM_BOUNDS, shapes);
      this.closedRooms.set(key, tile);
    }
    return tile;
  }
}
