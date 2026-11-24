/** Procedural rendering of the distorted cat death stab and low rumble. */

const TABLE = 4096;
const RELEASE = 1.5;
const TAU = Math.PI * 2;
const DETUNE = [0.9942404, 1, 1.0057929] as const;

interface DeathVoice {
  active: boolean;
  frame: number;
  frequency: number;
  gate: number;
  velocity: number;
  p0: number;
  p1: number;
  p2: number;
}

function createVoice(): DeathVoice {
  return {
    active: false,
    frame: 0,
    frequency: 0,
    gate: 0,
    velocity: 0,
    p0: 0,
    p1: 0,
    p2: 0,
  };
}

// Exponential approach that switches to a linear finish for the last tenth.
function approach(from: number, target: number, time: number, duration: number): number {
  if (time >= duration) return target;
  const tau = Math.log(duration + 1) / Math.log(200);
  const at = (t: number) => target + (from - target) * Math.exp(-t / tau);
  if (time < duration * 0.9) return at(time);
  return target + (at(duration * 0.9) - target) * ((duration - time) / (duration * 0.1));
}

function envelope(age: number, gate: number): number {
  const held = (time: number) => (time < 0.01 ? time / 0.01 : approach(1, 0.3, time - 0.01, 0.5));
  return age < gate ? held(age) : approach(held(gate), 0, age - gate, RELEASE);
}

export class Death {
  readonly sampleRate: number;
  readonly voices: DeathVoice[];
  private readonly saw: Float32Array;

  constructor(sampleRate = 48_000) {
    this.sampleRate = sampleRate;
    this.voices = Array.from({ length: 8 }, createVoice);
    const saw = new Float32Array(TABLE);
    let peak = 0;
    for (let index = 0; index < TABLE; index++) {
      const phase = (index / TABLE) * TAU;
      let sum = 0;
      for (let n = 1; n <= 10; n++) {
        sum += (Math.sin(phase * n) * (n % 2 === 0 ? -1 : 1)) / n;
      }
      saw[index] = sum;
      peak = Math.max(peak, Math.abs(sum));
    }
    if (peak > 0) {
      for (let index = 0; index < TABLE; index++) {
        saw[index] = saw[index]! / peak;
      }
    }
    this.saw = saw;
  }

  trigger(): void {
    const notes: readonly [number, number, number, number][] = [
      [51, 0.0625, 0, 0.45],
      [24, 2, 0.1, 0.55],
    ];
    for (const [midi, gate, delay, velocity] of notes) {
      let index = this.voices.findIndex((v) => !v.active);
      if (index === -1) {
        index = 0;
        let maxFrame = -Infinity;
        for (let i = 0; i < this.voices.length; i++) {
          const f = this.voices[i]!.frame;
          if (f > maxFrame) {
            maxFrame = f;
            index = i;
          }
        }
      }
      const v = this.voices[index]!;
      v.active = true;
      v.frame = -Math.trunc(delay * this.sampleRate);
      v.frequency = 440 * Math.pow(2, (midi - 69) / 12);
      v.gate = gate;
      v.velocity = velocity;
      v.p0 = 0;
      v.p1 = 2 / 3;
      v.p2 = 1 / 3;
    }
  }

  next(): number {
    let sum = 0;
    const sr = this.sampleRate;
    const saw = this.saw;
    for (let vi = 0; vi < this.voices.length; vi++) {
      const v = this.voices[vi]!;
      if (!v.active) continue;
      const age = v.frame / sr;
      v.frame += 1;
      if (age < 0) continue;
      if (age >= v.gate + RELEASE) {
        v.active = false;
        continue;
      }
      const env = envelope(age, v.gate) * v.velocity * 0.34276778;

      let pos = v.p0 * TABLE;
      let idx = pos | 0;
      let frac = pos - idx;
      idx = idx & (TABLE - 1);
      sum += (saw[idx]! + (saw[(idx + 1) & (TABLE - 1)]! - saw[idx]!) * frac) * env;
      v.p0 = (v.p0 + (v.frequency * DETUNE[0]) / sr) % 1;

      pos = v.p1 * TABLE;
      idx = pos | 0;
      frac = pos - idx;
      idx = idx & (TABLE - 1);
      sum += (saw[idx]! + (saw[(idx + 1) & (TABLE - 1)]! - saw[idx]!) * frac) * env;
      v.p1 = (v.p1 + (v.frequency * DETUNE[1]) / sr) % 1;

      pos = v.p2 * TABLE;
      idx = pos | 0;
      frac = pos - idx;
      idx = idx & (TABLE - 1);
      sum += (saw[idx]! + (saw[(idx + 1) & (TABLE - 1)]! - saw[idx]!) * frac) * env;
      v.p2 = (v.p2 + (v.frequency * DETUNE[2]) / sr) % 1;
    }
    const absSum = Math.abs(sum);
    if (absSum < 0.001) return 0;
    return ((83 * sum * 20 * Math.PI) / 180 / (Math.PI + 80 * absSum)) * 0.25118864;
  }
}
