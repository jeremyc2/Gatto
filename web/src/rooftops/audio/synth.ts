/**
 * Sample-clock procedural soundtrack and sound effects synthesizer.
 * Zero allocations during sample rendering.
 */

import { Death } from "./death";

const TAU = Math.PI * 2;
const REF_SAMPLE_RATE = 48_000;
const VOICES = 64;

export const FINALE = 3;
export const MENU = 4;

export const CUE_COIN = 1;
export const CUE_RETURN = 2;
export const CUE_SHUTTERS = 4;

const CHORDS: readonly (readonly number[])[] = [
  [57, 60, 64, 67, 71], // Am9
  [53, 57, 60, 64, 67], // Fmaj9
  [60, 64, 67, 71, 74], // Cmaj9
  [55, 59, 62, 65, 69], // G9
  [50, 53, 57, 60, 64], // Dm9
  [52, 55, 59, 62, 66], // Em9
  [53, 57, 60, 64, 69], // Fmaj7/A
  [55, 59, 64, 69, 74], // G6/9
];

const PROGRESSIONS: readonly (readonly number[])[] = [
  [0, 1, 2, 3, 4, 0, 6, 7],
  [0, 4, 1, 3, 2, 5, 6, 7],
  [2, 6, 0, 4, 1, 5, 4, 7],
  [0, 6, 4, 7, 2, 1, 5, 3],
];

const MOTIFS: readonly (readonly number[])[] = [
  [0, -1, -1, 2, -1, -1, 3, -1, 4, -1, 3, -1, 2, -1, -1, -1],
  [3, -1, -1, -1, 2, -1, 0, -1, 1, -1, -1, 2, -1, -1, 0, -1],
  [-1, -1, 2, -1, 4, -1, -1, 3, -1, -1, 2, -1, 0, -1, -1, -1],
  [4, -1, 3, -1, -1, -1, 2, -1, 1, -1, -1, -1, 0, -1, -1, 2],
];

const SCALE = [0, 3, 5, 7, 10, 12, 15] as const;

const HARBOR_CHORDS: readonly (readonly number[])[] = [
  [50, 57, 60, 64, 65], // Dm9
  [46, 53, 57, 60, 62], // Bbmaj9
  [53, 57, 60, 64, 67], // Fmaj9
  [48, 55, 58, 62, 64], // C9
  [43, 50, 57, 58, 65], // Gm9
  [45, 52, 55, 58, 61], // A7b9
  [52, 58, 62, 65, 67], // Em7b5
  [49, 55, 58, 64, 67], // C#dim7(add E)
];

const HARBOR_SECTIONS: readonly (readonly number[])[] = [
  [0, 1, 2, 3, 4, 1, 6, 5],
  [0, 4, 1, 2, 0, 3, 6, 5],
  [4, 3, 2, 1, 4, 6, 7, 5],
  [1, 2, 3, 0, 4, 1, 6, 5],
];

const HARBOR_MELODIES: readonly (readonly (number | null)[])[] = [
  [2, null, null, 3, 4, null],
  [null, 4, 3, null, 1, 2],
  [4, null, 2, 3, null, 1],
  [3, null, 2, null, null, 1],
];

const INST_KEYS = 0;
const INST_BASS = 1;
const INST_PAD = 2;
const INST_KICK = 3;
const INST_BRUSH = 4;
const INST_HAT = 5;
const INST_MALLET = 6;
const INST_BELL = 7;
const INST_CELESTA = 8;
const INST_COIN = 9;
const INST_COIN_BODY = 10;
const INST_CLICK = 11;

const LEAD_INSTRUMENTS = [INST_KEYS, INST_MALLET, INST_BELL, INST_CELESTA] as const;
const LEVEL_BPM = [88, 100, 108, 84] as const;
const LEVEL_TRANSPOSE = [0, 3, 5, 0] as const;
const COIN_CENTER_HZ = [9800, 9950, 9650, 9800] as const;
const SHUTTER_NOTE = [57, 61, 54, 57] as const;
const CLICK_DECAY = [110, 135, 90, 110] as const;

interface Voice {
  instrument: number;
  phase: number;
  frequency: number;
  age: number;
  duration: number;
  amplitude: number;
  pan: number;
  effect: boolean;
  nf0: number;
  nf1: number;
}

function createVoice(): Voice {
  return {
    instrument: INST_KEYS,
    phase: 0,
    frequency: 0,
    age: 0,
    duration: 0,
    amplitude: 0,
    pan: 0,
    effect: false,
    nf0: 0,
    nf1: 0,
  };
}

function adaptSmooth(coeff48k: number, sampleRate: number): number {
  if (sampleRate === REF_SAMPLE_RATE) return coeff48k;
  return 1 - Math.pow(1 - coeff48k, REF_SAMPLE_RATE / sampleRate);
}

class Preview {
  private readonly sampleRate: number;
  private readonly totalFrames: number;
  private readonly attackFrames: number;
  private readonly releaseFrames: number;
  remaining = 0;
  private phase = 0;
  private gain = 0;

  constructor(sampleRate: number) {
    this.sampleRate = sampleRate;
    this.totalFrames = Math.round((7200 * sampleRate) / REF_SAMPLE_RATE);
    this.attackFrames = Math.max(1, (192 * sampleRate) / REF_SAMPLE_RATE);
    this.releaseFrames = Math.max(1, (1200 * sampleRate) / REF_SAMPLE_RATE);
  }

  start(volume: number): void {
    const v = Math.max(0, Math.min(100, Math.round(volume)));
    this.remaining = v === 0 ? 0 : this.totalFrames;
    this.phase = 0;
    this.gain = (v / 100) * (v / 100);
  }

  next(): number {
    if (this.remaining <= 0) return 0;
    const attack = Math.min(1, (this.totalFrames - this.remaining) / this.attackFrames);
    const release = Math.min(1, this.remaining / this.releaseFrames);
    const sample = Math.sin(this.phase * TAU) * 0.16 * this.gain * attack * release;
    this.phase = (this.phase + 660 / this.sampleRate) % 1;
    this.remaining -= 1;
    return sample;
  }
}

export class Synth {
  readonly sampleRate: number;
  private readonly invSampleRate: number;
  private readonly voices: Voice[];
  private readonly delayL: Float32Array;
  private readonly delayR: Float32Array;
  private readonly delayLen: number;
  private delayCursor = 0;

  // Smoothing coefficients adapted to hardware sample rate.
  private readonly kMaster: number;
  private readonly kMusic: number;
  private readonly kEffects: number;
  private readonly kRocketGain: number;
  private readonly kRocketNoise: number;
  private readonly kCoinLow: number;
  private readonly kCoinHigh: number;
  private readonly kCoinBody: number;

  private rng: number;
  private readonly seed: number;
  level = 0;
  private targetLevel = 0;
  private resting = false;
  private bar = 0;
  private step = 0;
  private untilStep = 0;
  samples = 0;
  private master = 0;
  private musicGain = 0;
  private effectsGain = 0;
  private rocket = false;
  private rocketGain = 0;
  private rocketNoise = 0;
  private rocketPhase = 0;
  private finaleEntry = false;
  private readonly death: Death;
  private readonly preview: Preview;
  private readonly outPair: [number, number] = [0, 0];

  constructor(seed: number, sampleRate = REF_SAMPLE_RATE) {
    this.sampleRate = sampleRate;
    this.invSampleRate = 1 / sampleRate;
    this.voices = Array.from({ length: VOICES }, createVoice);
    this.delayLen = Math.max(1, Math.trunc(sampleRate * 0.37));
    this.delayL = new Float32Array(this.delayLen);
    this.delayR = new Float32Array(this.delayLen);

    this.kMaster = adaptSmooth(0.003, sampleRate);
    this.kMusic = adaptSmooth(0.0015, sampleRate);
    this.kEffects = adaptSmooth(0.003, sampleRate);
    this.kRocketGain = adaptSmooth(0.004, sampleRate);
    this.kRocketNoise = adaptSmooth(0.09, sampleRate);
    this.kCoinLow = adaptSmooth(0.18, sampleRate);
    this.kCoinHigh = adaptSmooth(0.78, sampleRate);
    this.kCoinBody = adaptSmooth(0.14, sampleRate);

    const s = Math.max(1, Math.abs(Math.trunc(seed)) | 0);
    this.rng = s >>> 0 || 1;
    this.seed = s % 97;
    this.death = new Death(sampleRate);
    this.preview = new Preview(sampleRate);
  }

  scene(level: number, resting: boolean): void {
    const clamped = Math.max(0, Math.min(MENU, level));
    if (clamped === FINALE && this.targetLevel !== FINALE) {
      this.finaleEntry = true;
    }
    this.targetLevel = clamped;
    this.resting = resting;
  }

  thruster(enabled: boolean): void {
    this.rocket = enabled;
  }

  previewEffects(volume: number): void {
    this.preview.start(volume);
  }

  previewing(): boolean {
    return this.preview.remaining > 0;
  }

  previewSample(): number {
    return this.preview.next();
  }

  private random(): number {
    // Fast allocation-free xorshift32.
    let x = this.rng;
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    this.rng = x >>> 0 || 1;
    return (this.rng >>> 8) / 16_777_216;
  }

  private note(
    instrument: number,
    midi: number,
    duration: number,
    amplitude: number,
    pan: number,
    delay: number,
    effect: boolean,
  ): void {
    let index = -1;
    let minRem = Infinity;
    let minIdx = 0;
    for (let i = 0; i < VOICES; i++) {
      const v = this.voices[i]!;
      const rem = v.duration - v.age;
      if (rem <= 0) {
        index = i;
        break;
      }
      if (rem < minRem) {
        minRem = rem;
        minIdx = i;
      }
    }
    const v = this.voices[index === -1 ? minIdx : index]!;
    v.instrument = instrument;
    v.phase = 0;
    v.frequency = 440 * Math.pow(2, (midi - 69) / 12);
    v.age = -delay;
    v.duration = duration;
    v.amplitude = amplitude;
    v.pan = pan < -1 ? -1 : pan > 1 ? 1 : pan;
    v.effect = effect;
    v.nf0 = 0;
    v.nf1 = 0;
  }

  cues(bits: number): void {
    if ((bits & CUE_COIN) !== 0) {
      const center = COIN_CENTER_HZ[Math.min(this.targetLevel, FINALE)]!;
      const hits: readonly [number, number][] = [
        [0, 1],
        [0.09, 0.47],
        [0.14, 0.45],
        [0.21, 0.18],
        [0.265, 0.24],
        [0.35, 0.07],
      ];
      const bodyNote = 69 + 12 * Math.log2(565 / 440);
      for (const [delay, strength] of hits) {
        const metalHz = center * (0.996 + this.random() * 0.008);
        const metalNote = 69 + 12 * Math.log2(metalHz / 440);
        this.note(INST_COIN, metalNote, 0.095, 0.12 * strength, 0.03, delay, true);
        this.note(INST_COIN_BODY, bodyNote, 0.045, 0.078 * strength, 0, delay, true);
      }
    }
    if ((bits & CUE_RETURN) !== 0) {
      this.death.trigger();
    }
    if ((bits & CUE_SHUTTERS) !== 0) {
      const note = SHUTTER_NOTE[Math.min(this.targetLevel, FINALE)]!;
      this.note(INST_CLICK, note, 0.065, 0.13, 0, 0, true);
      this.note(INST_CLICK, note, 0.065, 0.075, 0, 0.024, true);
      this.note(INST_CLICK, note, 0.065, 0.045, 0, 0.052, true);
    }
  }

  private schedule(): void {
    if (this.step === 0) {
      if (this.level !== this.targetLevel) {
        for (let i = 0; i < VOICES; i++) {
          const v = this.voices[i]!;
          if (!v.effect && v.age < v.duration) {
            v.duration = Math.min(v.duration, Math.max(v.age, 0) + 0.08);
          }
        }
      }
      this.level = this.targetLevel;
    }
    if (this.level === MENU) {
      this.scheduleHarbor();
      return;
    }
    const beat = 60 / LEVEL_BPM[this.level]!;
    const transpose = LEVEL_TRANSPOSE[this.level]!;
    const cycle = Math.floor(this.bar / 64);
    const section = Math.floor(this.bar / 8) % 8;
    const progression =
      this.level === 3 ? 2 : (cycle + this.level + this.seed) % PROGRESSIONS.length;

    if (this.finaleEntry && this.step === 0) {
      this.finaleEntry = false;
      const arp = [72, 76, 79, 83, 86];
      for (let index = 0; index < arp.length; index++) {
        this.note(INST_CELESTA, arp[index]!, 1.8, 0.035, index * 0.15 - 0.3, index * 0.13, false);
      }
    }

    const chord = CHORDS[PROGRESSIONS[progression]![Math.floor(this.bar / 2) % 8]!]!;
    const sparse = section === 0 || section === 3 || this.resting;

    if (this.step === 0 || (!sparse && this.step === 10)) {
      const inversion = (Math.floor(this.bar / 8) + cycle) % 3;
      for (let index = 0; index < chord.length; index++) {
        const octave = index < inversion ? 12 : 0;
        const velocity = 0.024 * (0.9 + this.random() * 0.2);
        this.note(
          INST_KEYS,
          chord[index]! + transpose + octave,
          beat * (this.step === 0 ? 3.4 : 1.4),
          velocity,
          index * 0.22 - 0.44,
          index * 0.009,
          false,
        );
      }
    }

    if (this.level >= 2 && this.step === 0 && this.bar % 2 === 0) {
      this.note(INST_PAD, chord[1]! + transpose + 12, beat * 7, 0.012, 0.3, 0, false);
      this.note(INST_PAD, chord[3]! + transpose + 12, beat * 7, 0.012, 0.3, 0, false);
    }

    if (this.step === 0 || this.step === 8 || (!sparse && (this.step === 6 || this.step === 14))) {
      const note = this.step === 6 ? chord[2]! : this.step === 14 ? chord[1]! : chord[0]!;
      this.note(INST_BASS, note + transpose - 12, beat * 0.85, 0.095, -0.08, 0, false);
    }

    if (
      (!sparse || (section === 0 && this.bar % 4 === 2) || this.level === 3) &&
      section !== 7 &&
      (!this.resting || this.level === 3)
    ) {
      const motif = (section + (this.bar % 4) + cycle + this.seed) % MOTIFS.length;
      const degree = MOTIFS[motif]![this.step]!;
      if (degree >= 0 && (this.bar % 4 !== 3 || this.step < 8)) {
        const octave = section === 5 || this.level === 2 ? 12 : 0;
        const note = 69 + transpose + SCALE[degree]! + octave;
        const velocity = 0.034 * (0.8 + this.random() * 0.3);
        this.note(
          LEAD_INSTRUMENTS[this.level]!,
          note,
          beat * 0.8,
          velocity,
          this.bar % 2 === 0 ? -0.28 : 0.28,
          0,
          false,
        );
      }
    }

    if (this.step === 0 || this.step === 8) {
      this.note(INST_KICK, 36, 0.22, 0.105, 0, 0, false);
    }
    if (this.step === 4 || this.step === 12) {
      this.note(INST_BRUSH, 52, 0.16, sparse ? 0.025 : 0.045, 0.18, 0, false);
    }
    if (this.step % 4 === 2 || (!sparse && this.level > 0 && this.step % 4 === 0)) {
      const velocity = 0.018 * (0.7 + this.random() * 0.5);
      this.note(INST_HAT, 90, 0.055, velocity, -0.35, 0, false);
    }
    if (!sparse && this.bar % 8 === 7 && (this.step === 13 || this.step === 15)) {
      this.note(INST_BRUSH, 57, 0.11, 0.025, -0.2, 0, false);
    }
    const swing = this.step % 4 < 2 ? 0.28 : 0.22;
    this.advanceStep(16, beat * swing);
  }

  private scheduleHarbor(): void {
    const BEAT = 60 / 72;
    const section = Math.floor(this.bar / 16) % HARBOR_SECTIONS.length;
    const chorus = Math.floor(this.bar / 64);
    const bar = this.bar % 16;
    const chord = HARBOR_CHORDS[HARBOR_SECTIONS[section]![Math.floor(bar / 2)]!]!;
    const driving = section === 1 || section === 2;

    if (this.step === 0 || (this.step === 4 && driving)) {
      const note = chord[this.step === 0 ? 0 : 1]!;
      this.note(INST_BASS, note - 12, BEAT * 1.9, 0.085, -0.16, 0, false);
    }
    if ((this.step === 0 && bar % 2 === 0) || (this.step === 2 && driving)) {
      for (let index = 0; index < chord.length - 1; index++) {
        this.note(
          INST_KEYS,
          chord[index + 1]! + 12,
          BEAT * 2.8,
          0.036,
          index * 0.24 - 0.36,
          index * 0.032,
          false,
        );
      }
    }
    const phrase = (section + Math.floor(bar / 4) + chorus) % HARBOR_MELODIES.length;
    const degree = HARBOR_MELODIES[phrase]![this.step] ?? null;
    if ((driving || bar % 4 !== 3) && degree !== null) {
      const note = chord[degree]! + (section === 2 ? 24 : 12);
      const accent = 0.041 + this.random() * 0.009;
      this.note(
        section === 2 ? INST_BELL : INST_CELESTA,
        note,
        BEAT * 1.55,
        accent,
        0.31,
        0.024,
        false,
      );
    }
    if (this.step === 0 && bar % 2 === 0) {
      this.note(INST_PAD, chord[2]!, BEAT * 5.9, 0.025, -0.24, 0, false);
      this.note(INST_PAD, chord[3]!, BEAT * 5.9, 0.025, -0.24, 0, false);
    }
    if (driving && (this.step === 2 || this.step === 4)) {
      this.note(INST_BRUSH, 51, 0.21, 0.024, -0.34, 0.032, false);
    }
    if (bar % 4 === 3 && this.step === 5) {
      this.note(INST_MALLET, chord[1]! + 12, BEAT * 0.8, 0.028, -0.48, 0, false);
    }
    const lilt = this.step % 2 === 0 ? 0.53 : 0.47;
    this.advanceStep(6, BEAT * lilt);
  }

  private advanceStep(steps: number, seconds: number): void {
    this.untilStep += seconds * this.sampleRate;
    this.step += 1;
    if (this.step === steps) {
      this.step = 0;
      this.bar += 1;
    }
  }

  next(active: boolean, music: number, effects: number): [number, number] {
    this.master += ((active ? 1 : 0) - this.master) * this.kMaster;
    if (!active && this.master < 0.0001) {
      this.master = 0;
      this.outPair[0] = 0;
      this.outPair[1] = 0;
      return this.outPair;
    }
    const mNorm = Math.min(100, Math.max(0, music)) / 100;
    const eNorm = Math.min(100, Math.max(0, effects)) / 100;
    const musicTarget = mNorm * mNorm * (this.resting ? 0.52 : 1);
    const effectsTarget = eNorm * eNorm;
    this.musicGain += (musicTarget - this.musicGain) * this.kMusic;
    this.effectsGain += (effectsTarget - this.effectsGain) * this.kEffects;

    if (this.untilStep <= 0) {
      this.schedule();
    }
    this.untilStep -= 1;
    this.samples += 1;

    const noise = this.random() * 2 - 1;
    let musicL = 0;
    let musicR = 0;
    let fxL = 0;
    let fxR = 0;
    const invSr = this.invSampleRate;
    const clickDecay = CLICK_DECAY[Math.min(this.level, FINALE)]!;

    for (let i = 0; i < VOICES; i++) {
      const v = this.voices[i]!;
      if (v.age >= v.duration) continue;
      v.age += invSr;
      if (v.age < 0) continue;
      const age = v.age;
      const inst = v.instrument;
      const frequency = inst === INST_KICK ? 46 + 92 * Math.exp(-age * 32) : v.frequency;
      v.phase = (v.phase + frequency * invSr) % 1;
      const phase = v.phase * TAU;
      const attackRate =
        inst === INST_COIN || inst === INST_CLICK || inst === INST_COIN_BODY ? 2400 : 160;
      const attack = age * attackRate < 1 ? age * attackRate : 1;
      let release = (v.duration - age) * 30;
      if (release < 0) release = 0;
      else if (release > 1) release = 1;

      let wave = 0;
      switch (inst) {
        case INST_KEYS:
          wave =
            (Math.sin(phase) +
              Math.sin(phase * 2) * 0.25 * Math.exp(-age * 6) +
              Math.sin(phase * 3) * 0.1 * Math.exp(-age * 10)) *
            Math.exp(-age * 2.5);
          break;
        case INST_BASS:
          wave = (Math.sin(phase) + Math.sin(phase * 2) * 0.17) * Math.exp(-age * 2.8);
          break;
        case INST_PAD: {
          let padRel = (v.duration - age) * 1.4;
          if (padRel < 0) padRel = 0;
          else if (padRel > 1) padRel = 1;
          wave =
            (Math.sin(phase) * 0.65 + Math.sin(phase + age * v.frequency * 0.002 * TAU) * 0.35) *
            Math.min(1, age * 2) *
            padRel;
          break;
        }
        case INST_KICK:
          wave = Math.sin(phase) * Math.exp(-age * 21);
          break;
        case INST_BRUSH:
          wave = (noise * 0.8 + Math.sin(phase) * 0.2) * Math.exp(-age * 28);
          break;
        case INST_HAT:
          wave = noise * Math.exp(-age * 100);
          break;
        case INST_MALLET:
          wave =
            (Math.sin(phase) + Math.sin(phase * 3) * 0.28 * Math.exp(-age * 16)) *
            Math.exp(-age * 11);
          break;
        case INST_BELL:
          wave =
            (Math.sin(phase) +
              Math.sin(age * v.frequency * 2.76 * TAU) * 0.22 * Math.exp(-age * 8)) *
            Math.exp(-age * 4.2);
          break;
        case INST_CELESTA:
          wave =
            (Math.sin(phase) +
              Math.sin(phase * 2) * 0.18 +
              Math.sin(phase * 4) * 0.08 * Math.exp(-age * 6)) *
            Math.exp(-age * 3.8);
          break;
        case INST_COIN: {
          const strike = age * v.frequency * TAU;
          v.nf0 += (noise - v.nf0) * this.kCoinLow;
          const high = noise - v.nf0;
          v.nf1 += (high - v.nf1) * this.kCoinHigh;
          wave =
            Math.sin(strike) * 0.45 * Math.exp(-age * 65) +
            Math.sin(strike * (7300 / 9800)) * 0.34 * Math.exp(-age * 85) +
            Math.sin(strike * (12100 / 9800)) * 0.2 * Math.exp(-age * 110) +
            Math.sin(strike * 0.993) * 0.12 * Math.exp(-age * 75) +
            Math.sin(strike * 1.007) * 0.1 * Math.exp(-age * 90) +
            v.nf1 * 0.45 * Math.exp(-age * 60);
          break;
        }
        case INST_COIN_BODY: {
          const contact = age * v.frequency * TAU;
          v.nf0 += (noise - v.nf0) * this.kCoinBody;
          wave =
            Math.sin(contact) * 0.6 * Math.exp(-age * 140) +
            Math.sin(contact * 1.604) * 0.25 * Math.exp(-age * 180) +
            Math.sin(contact * 1.9) * 0.18 * Math.exp(-age * 220) +
            v.nf0 * 0.4 * Math.exp(-age * 180);
          break;
        }
        case INST_CLICK:
          wave = (noise * 0.6 + Math.sin(phase) * 0.4) * Math.exp(-age * clickDecay);
          break;
      }

      const sample = wave * attack * release * v.amplitude;
      const panL = 1 - v.pan * 0.5;
      const panR = 1 + v.pan * 0.5;
      if (v.effect) {
        fxL += sample * panL;
        fxR += sample * panR;
      } else {
        musicL += sample * panL;
        musicR += sample * panR;
      }
    }

    this.rocketGain += ((this.rocket ? 1 : 0) - this.rocketGain) * this.kRocketGain;
    this.rocketNoise += (noise - this.rocketNoise) * this.kRocketNoise;
    this.rocketPhase = (this.rocketPhase + 74 * invSr) % 1;
    const rocket =
      (Math.sin(this.rocketPhase * TAU) * 0.032 + this.rocketNoise * 0.11) * this.rocketGain;
    fxL += rocket;
    fxR += rocket * 0.94;

    const dL = this.delayL[this.delayCursor]!;
    const dR = this.delayR[this.delayCursor]!;
    this.delayL[this.delayCursor] = musicL + dR * 0.24;
    this.delayR[this.delayCursor] = musicR + dL * 0.24;
    this.delayCursor += 1;
    if (this.delayCursor === this.delayLen) this.delayCursor = 0;

    const death = this.death.next() * this.effectsGain * this.master;

    const mixL =
      ((musicL + dL * 0.17) * this.musicGain + fxL * this.effectsGain) * this.master * 1.8;
    const mixR =
      ((musicR + dR * 0.17) * this.musicGain + fxR * this.effectsGain) * this.master * 1.8;
    let outL = (mixL / (1 + Math.abs(mixL))) * 0.85 + death;
    let outR = (mixR / (1 + Math.abs(mixR))) * 0.85 + death;
    if (outL > 0.95) outL = 0.95;
    else if (outL < -0.95) outL = -0.95;
    if (outR > 0.95) outR = 0.95;
    else if (outR < -0.95) outR = -0.95;

    this.outPair[0] = outL;
    this.outPair[1] = outR;
    return this.outPair;
  }

  renderBlock(
    outL: Float32Array,
    outR: Float32Array,
    active: boolean,
    music: number,
    effects: number,
  ): void {
    const len = outL.length;
    for (let i = 0; i < len; i++) {
      const frame = this.next(active, music, effects);
      const prev = this.preview.next();
      outL[i] = frame[0] + prev;
      outR[i] = frame[1] + prev;
    }
  }
}
