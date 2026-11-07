import { join } from "node:path";

// Original, deterministic Foley for the annotation film. No samples or dependencies.
// bun scripts/generate-demo-audio.ts [output-directory] [timeline.json]
// Timeline: { duration: 14, masterGain?: 0.316227766,
//   events: [{ cue: "tool"|"draw"|"place"|"camera"|"type",
//   time: 1, duration?: 1, gain?: 1, pan?: 0 }] }
const rate = 48_000;
const output = process.argv[2] ?? "/tmp/gatto-demo-audio";
const kinds = ["tool", "draw", "place", "camera", "type"] as const;
type Kind = (typeof kinds)[number];
type Event = { cue: Kind; time: number; duration?: number; gain?: number; pan?: number };
type Timeline = { duration: number; masterGain?: number; events: Event[] };
// Ten decibels below the first cut; event gains retain the same relative balance.
const defaultMasterGain = 10 ** (-10 / 20);
const defaults: Record<Kind, number> = {
  tool: 0.12,
  draw: 1.2,
  place: 0.22,
  camera: 0.6,
  type: 0.07,
};
const defaultTimeline: Timeline = {
  duration: 14,
  events: [
    { cue: "tool", time: 1.15, pan: -0.3 },
    { cue: "camera", time: 1.4, duration: 0.8, gain: 0.4 },
    { cue: "tool", time: 2.2, gain: 0.4, pan: 0.08 },
    { cue: "draw", time: 2.2, duration: 1.8, gain: 0.9, pan: 0.12 },
    { cue: "place", time: 4, pan: 0.15 },
    { cue: "tool", time: 5.05, pan: -0.15 },
    { cue: "tool", time: 6.8, gain: 0.4, pan: 0.15 },
    { cue: "draw", time: 6.8, duration: 2.4, gain: 0.8, pan: 0.12 },
    { cue: "place", time: 9.2, pan: -0.12 },
    { cue: "camera", time: 10.2, duration: 2, gain: 0.35 },
  ],
};

function noise(seed: number) {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 2 ** 31 - 1;
  };
}

function cue(kind: Kind, duration = defaults[kind]): Float32Array {
  const count = Math.round(duration * rate);
  const samples = new Float32Array(count);
  const random = noise(83413 + kinds.indexOf(kind) * 1009);
  let low = 0;
  let slow = 0;
  let previous = 0;
  for (let i = 0; i < count; i++) {
    const t = i / rate;
    const u = i / count;
    const n = random();
    low += (n - low) * 0.13;
    slow += (n - slow) * 0.018;
    const grain = low - slow;
    const fade = Math.min(1, t / 0.012, (duration - t) / 0.035);
    let sample = 0;
    if (kind === "tool") {
      // A padded, miniature mechanical click, rather than a digital tick.
      const impact = Math.exp(-t * 60) * (1 - Math.exp(-t * 1600));
      sample = impact * (grain * 0.27 + Math.sin(t * Math.PI * 2 * 460) * 0.015);
    } else if (kind === "type") {
      // Tiny soft key contact: shorter and quieter than a tool-selection tap.
      const impact = Math.exp(-t * 95) * (1 - Math.exp(-t * 1400));
      sample = impact * grain * 0.075;
    } else if (kind === "place") {
      // A short surface contact with an almost inaudible low body.
      const impact = Math.exp(-t * 33) * (1 - Math.exp(-t * 1000));
      sample = impact * (grain * 0.2 + Math.sin(t * Math.PI * 2 * 180) * 0.036);
    } else if (kind === "draw") {
      // Soft graphite grain with slow pressure changes and no repetitive loop.
      const pressure = 0.65 + Math.sin(u * 13 + Math.sin(u * 5)) * 0.16;
      const texture = grain * 0.72 + (n - previous) * 0.025;
      sample = texture * Math.pow(Math.sin(Math.PI * u), 0.35) * pressure * 0.13;
    } else {
      // Breath-like movement, kept below the drawing and placement sounds.
      sample = slow * Math.pow(Math.sin(Math.PI * u), 2) * 0.3;
    }
    samples[i] = sample * Math.max(0, fade) * 2.5;
    previous = n;
  }
  return samples;
}

function mix(duration: number, events: Event[], masterGain: number): Float32Array {
  const data = new Float32Array(Math.ceil(duration * rate) * 2);
  for (const event of events) {
    if (
      !kinds.includes(event.cue) ||
      !Number.isFinite(event.time) ||
      event.time < 0 ||
      (event.duration !== undefined && (!Number.isFinite(event.duration) || event.duration <= 0)) ||
      (event.gain !== undefined && (!Number.isFinite(event.gain) || event.gain < 0)) ||
      (event.pan !== undefined && !Number.isFinite(event.pan))
    )
      throw new Error("Invalid sound event");
    const source = cue(event.cue, event.duration);
    const pan = Math.max(-1, Math.min(1, event.pan ?? 0));
    const gain = (event.gain ?? 1) * masterGain;
    const left = Math.cos(((pan + 1) * Math.PI) / 4) * gain;
    const right = Math.sin(((pan + 1) * Math.PI) / 4) * gain;
    const start = Math.round(event.time * rate);
    for (let i = 0; i < source.length && start + i < data.length / 2; i++) {
      const index = (start + i) * 2;
      const sample = source[i] ?? 0;
      data[index] = (data[index] ?? 0) + sample * left;
      data[index + 1] = (data[index + 1] ?? 0) + sample * right;
    }
  }
  return data;
}

function wav(data: Float32Array): Uint8Array {
  const bytes = new Uint8Array(44 + data.length * 2);
  const view = new DataView(bytes.buffer);
  const ascii = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) bytes[offset + i] = value.charCodeAt(i);
  };
  ascii(0, "RIFF");
  view.setUint32(4, bytes.length - 8, true);
  ascii(8, "WAVE");
  ascii(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 2, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 16, true);
  ascii(36, "data");
  view.setUint32(40, data.length * 2, true);
  for (let i = 0; i < data.length; i++) {
    view.setInt16(44 + i * 2, Math.round(Math.max(-1, Math.min(1, data[i] ?? 0)) * 32767), true);
  }
  return bytes;
}

const timeline = process.argv[3]
  ? ((await Bun.file(process.argv[3]).json()) as Timeline)
  : defaultTimeline;
if (!Number.isFinite(timeline.duration) || timeline.duration <= 0) {
  throw new Error("Timeline duration must be positive");
}
const masterGain = timeline.masterGain ?? defaultMasterGain;
if (!Number.isFinite(masterGain) || masterGain < 0) {
  throw new Error("Master gain must be a finite, non-negative value");
}
for (const kind of kinds) {
  const data = mix(defaults[kind], [{ cue: kind, time: 0 }], masterGain);
  await Bun.write(join(output, `${kind}.wav`), wav(data));
}
const data = mix(timeline.duration, timeline.events, masterGain);
await Bun.write(join(output, "annotation-foley.wav"), wav(data));
await Bun.write(join(output, "mix.wav"), wav(data));
await Bun.write(
  join(output, "timeline.json"),
  `${JSON.stringify({ ...timeline, masterGain }, null, 2)}\n`,
);
console.log(`Wrote original stereo 48 kHz Foley to ${output}`);
