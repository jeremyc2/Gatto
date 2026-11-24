import { expect, test } from "bun:test";
import { flightPose, projectReturnPose, returnPose } from "../src/portal/flight";
import { ease, mix } from "../src/portal/geometry";
import { sceneryProjection } from "../src/portal/scenery";

const frame = { width: 503, height: 394, x: 251.5, y: 197, radius: 114, time: 5.65 };

test("the outbound flight cruises continuously through the aperture", () => {
  const poses = Array.from({ length: 15 }, (_, index) =>
    flightPose({ ...frame, time: 2.25 + index * 0.1 }),
  );
  const speeds = poses.slice(1).map((pose, index) => {
    const previous = poses[index]!;
    return Math.hypot(pose.x - previous.x, pose.y - previous.y);
  });
  expect(Math.max(...speeds) / Math.min(...speeds)).toBeLessThan(1.08);
  expect(poses[0]!.crossing).toBe(0);
  expect(poses.at(-1)!.crossing).toBe(1);
});

test("the return keeps the starting pose and holds the rear view behind the rim", () => {
  const initial = flightPose(frame);
  const start = returnPose(frame, frame.time, 0);
  expect([start.x, start.y, start.size, start.crossing]).toEqual([
    initial.x,
    initial.y,
    initial.size,
    initial.crossing,
  ]);
  expect(returnPose(frame, frame.time, 0.3).turn).toBe(6);
  const receding = returnPose(frame, frame.time, 0.9);
  expect(receding.turn).toBeGreaterThanOrEqual(18);
  expect(receding.size).toBeLessThan(start.size);
  expect(returnPose(frame, frame.time, 1.7).turn).toBeGreaterThanOrEqual(18);
  const end = returnPose(frame, frame.time, 1.98);
  expect(end.crossing).toBe(0);
  expect(end.visibility).toBe(1);
  expect(end.x).toBeGreaterThan(0);
});

test("the cat brakes beyond the aperture and then holds its position", () => {
  const beforeBrake = returnPose(frame, frame.time, 1.54);
  const braking = returnPose(frame, frame.time, 1.62);
  const stopped = returnPose(frame, frame.time, 1.7);
  for (const elapsed of [0.8, 1.1, 1.4]) {
    const pose = returnPose(frame, frame.time, elapsed);
    expect(pose.x * pose.depth).toBeCloseTo(stopped.x * stopped.depth);
    expect(pose.y * pose.depth).toBeCloseTo(stopped.y * stopped.depth);
  }
  expect(stopped.depth - braking.depth).toBeLessThan(braking.depth - beforeBrake.depth);
  const later = returnPose(frame, frame.time, 2.2);
  expect([later.depth, later.x, later.y, later.size]).toEqual([
    stopped.depth,
    stopped.x,
    stopped.y,
    stopped.size,
  ]);
  expect(later.crossing).toBe(0);
});

test("a forward camera pulls level with the stopped cat", () => {
  for (const [width, height] of [
    [1440, 900],
    [503, 780],
    [740, 335],
  ] as const) {
    const radius = Math.min(frame.radius, height * 0.29);
    const project = (elapsed: number) => {
      const approach = ease(0.62, 2.3, elapsed);
      const y = mix(height * 0.69, height / 2, approach);
      const scale = (Math.hypot(width, height) / (radius * 1.72)) ** approach;
      return projectReturnPose(
        returnPose({ ...frame, radius }, frame.time, elapsed),
        { x: width / 2, y, scale, width, hasTurn: true },
        elapsed,
      );
    };
    const ahead = project(1.7);
    const closer = project(1.93);
    expect(ahead).toBeDefined();
    expect(closer).toBeDefined();
    expect(closer!.size).toBeGreaterThan(ahead!.size);
    expect(closer!.visibility).toBe(1);
    // Size and horizontal offset grow by the same perspective factor.
    expect((closer!.x - width / 2) / closer!.size).toBeCloseTo(
      (ahead!.x - width / 2) / ahead!.size,
    );
    expect(project(2.26)).toBeUndefined();
  }
});

test("the aperture opens out into the full harbor at its native viewport scale", () => {
  for (const [width, height] of [
    [1440, 900],
    [503, 780],
    [740, 335],
  ] as const) {
    const radius = 0.24 * Math.min(width, height);
    const start = sceneryProjection(width, height, radius, width / 2, height / 2, 0);
    const full = sceneryProjection(width, height, radius, width / 2, height / 2, 1);
    expect(start.scale).toBeLessThanOrEqual(1);
    expect(full).toEqual({ x: 0, y: 0, scale: 1 });
    const maxZoom = Math.hypot(width, height) / (radius * 1.72);
    let previous = 0;
    for (let i = 0; i <= 10; i++) {
      const t = i / 10;
      const expanded = radius * maxZoom ** t;
      const x = width / 2;
      const y = height * (0.69 - 0.19 * t);
      const projection = sceneryProjection(width, height, radius, x, y, t, expanded);
      const viewRadius = expanded / projection.scale;
      const edge = expanded * 1.05;
      expect(projection.x).toBeLessThanOrEqual(Math.max(0, x - edge) + 1e-8);
      expect(projection.y).toBeLessThanOrEqual(Math.max(0, y - edge) + 1e-8);
      expect(projection.x + width * projection.scale).toBeGreaterThanOrEqual(
        Math.min(width, x + edge) - 1e-8,
      );
      expect(projection.y + height * projection.scale).toBeGreaterThanOrEqual(
        Math.min(height, y + edge) - 1e-8,
      );
      expect(viewRadius).toBeGreaterThanOrEqual(previous - 1e-8);
      previous = viewRadius;
    }
  }
});
