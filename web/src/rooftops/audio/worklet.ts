import { Synth } from "./synth";

declare class AudioWorkletProcessor {
  readonly port: MessagePort;
  process(
    inputs: Float32Array[][],
    outputs: Float32Array[][],
    parameters: Record<string, Float32Array>,
  ): boolean;
}

declare function registerProcessor(name: string, processorCtor: typeof AudioWorkletProcessor): void;
declare const sampleRate: number;

type WorkletMessage =
  | {
      type: "sync";
      active: boolean;
      music: number;
      effects: number;
      level: number;
      resting: boolean;
      thruster: boolean;
    }
  | { type: "cue"; bits: number }
  | { type: "preview"; volume: number }
  | { type: "suspend" }
  | { type: "reset"; seed: number };

class RooftopsAudioProcessor extends AudioWorkletProcessor {
  private synth: Synth;
  private active = false;
  private music = 75;
  private effects = 55;

  constructor() {
    super();
    this.synth = new Synth(Date.now() & 0x7fffffff, sampleRate);
    this.port.onmessage = (event: MessageEvent<WorkletMessage>) => {
      const msg = event.data;
      switch (msg.type) {
        case "sync":
          this.active = msg.active;
          this.music = msg.music;
          this.effects = msg.effects;
          this.synth.scene(msg.level, msg.resting);
          this.synth.thruster(msg.thruster);
          break;
        case "cue":
          this.synth.cues(msg.bits);
          break;
        case "preview":
          this.synth.previewEffects(msg.volume);
          break;
        case "suspend":
          this.active = false;
          this.synth.previewEffects(0);
          break;
        case "reset":
          this.active = false;
          this.synth = new Synth(msg.seed, sampleRate);
          break;
      }
    };
  }

  override process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    const output = outputs[0];
    if (!output) return true;
    const left = output[0];
    if (!left) return true;
    const right = output[1];
    if (right) {
      this.synth.renderBlock(left, right, this.active, this.music, this.effects);
    } else {
      for (let i = 0; i < left.length; i++) {
        const frame = this.synth.next(this.active, this.music, this.effects);
        const prev = this.synth.previewSample();
        left[i] = (frame[0] + frame[1]) * 0.5 + prev;
      }
    }
    return true;
  }
}

registerProcessor("rooftops-synth", RooftopsAudioProcessor);
