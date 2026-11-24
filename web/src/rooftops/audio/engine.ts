import workletUrl from "./worklet.ts?worker&url";
import { CUE_COIN, CUE_RETURN, CUE_SHUTTERS, FINALE, MENU } from "./synth";
import type { Preferences } from "../types";

export { CUE_COIN, CUE_RETURN, CUE_SHUTTERS, FINALE, MENU };

export interface AudioScene {
  active: boolean;
  level: number;
  resting: boolean;
  thruster: boolean;
}

export class AudioEngine {
  private ctx: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  private initializing = false;
  private unavailable = false;
  private active = false;
  private lastSync: {
    active: boolean;
    music: number;
    effects: number;
    level: number;
    resting: boolean;
    thruster: boolean;
  } = {
    active: false,
    music: 75,
    effects: 55,
    level: MENU,
    resting: false,
    thruster: false,
  };
  private pendingPreview: number | null = null;
  private onStatusChange: (() => void) | null = null;

  setStatusListener(listener: () => void): void {
    this.onStatusChange = listener;
  }

  error(): string | null {
    return this.unavailable ? "Audio is unavailable on this device." : null;
  }

  /** Call from user gestures to satisfy browser autoplay policy. */
  unlock(): void {
    if (this.unavailable) return;
    if (!this.ctx && !this.initializing) {
      void this.init();
      return;
    }
    if (
      this.ctx &&
      this.ctx.state === "suspended" &&
      (this.active || this.pendingPreview !== null)
    ) {
      void this.ctx.resume().catch(() => {});
    }
  }

  sync(scene: AudioScene, preferences: Preferences): void {
    const active = scene.active && (preferences.musicVolume > 0 || preferences.effectsVolume > 0);
    const next = {
      active,
      music: preferences.musicVolume,
      effects: preferences.effectsVolume,
      level: scene.level,
      resting: scene.resting,
      thruster: scene.thruster,
    };
    const changed =
      next.active !== this.lastSync.active ||
      next.music !== this.lastSync.music ||
      next.effects !== this.lastSync.effects ||
      next.level !== this.lastSync.level ||
      next.resting !== this.lastSync.resting ||
      next.thruster !== this.lastSync.thruster;
    this.lastSync = next;
    this.active = active;

    if (active && !this.ctx && !this.initializing && !this.unavailable) {
      void this.init();
    }
    if (this.node && changed) {
      this.node.port.postMessage({ type: "sync", ...next });
    }
    if (this.ctx && active && this.ctx.state === "suspended") {
      void this.ctx.resume().catch(() => {});
    }
  }

  cue(bits: number): void {
    if (!this.active || this.lastSync.effects <= 0) return;
    if (this.node) {
      this.node.port.postMessage({ type: "cue", bits });
    }
  }

  previewEffects(volume: number): void {
    const v = Math.max(0, Math.min(100, Math.round(volume)));
    if (v > 0 && this.unavailable) {
      this.unavailable = false;
    }
    if (!this.ctx && !this.initializing && v > 0) {
      this.pendingPreview = v;
      void this.init();
      return;
    }
    if (this.ctx && v > 0 && this.ctx.state === "suspended") {
      void this.ctx.resume().catch(() => {});
    }
    if (this.node) {
      this.node.port.postMessage({ type: "preview", volume: v });
    } else if (v > 0) {
      this.pendingPreview = v;
    }
  }

  suspend(): void {
    this.active = false;
    this.lastSync = { ...this.lastSync, active: false };
    this.pendingPreview = null;
    if (this.node) {
      this.node.port.postMessage({ type: "suspend" });
    }
  }

  stop(): void {
    this.suspend();
    if (this.node) {
      this.node.port.postMessage({ type: "reset", seed: Date.now() & 0x7fffffff });
    }
  }

  retry(): void {
    if (this.unavailable) {
      this.unavailable = false;
      this.onStatusChange?.();
    }
    if (!this.ctx && !this.initializing) {
      void this.init();
    }
  }

  dispose(): void {
    this.suspend();
    if (this.node) {
      this.node.disconnect();
      this.node = null;
    }
    if (this.ctx) {
      void this.ctx.close().catch(() => {});
      this.ctx = null;
    }
  }

  private async init(): Promise<void> {
    if (this.initializing || this.ctx) return;
    if (typeof AudioContext === "undefined" || typeof AudioWorkletNode === "undefined") {
      this.unavailable = true;
      this.onStatusChange?.();
      return;
    }
    this.initializing = true;
    try {
      let ctx: AudioContext;
      try {
        ctx = new AudioContext({ sampleRate: 48_000, latencyHint: "interactive" });
      } catch {
        ctx = new AudioContext({ latencyHint: "interactive" });
      }
      await ctx.audioWorklet.addModule(workletUrl);
      const node = new AudioWorkletNode(ctx, "rooftops-synth", {
        numberOfInputs: 0,
        numberOfOutputs: 1,
        outputChannelCount: [2],
      });
      node.connect(ctx.destination);
      this.ctx = ctx;
      this.node = node;
      this.unavailable = false;
      node.port.postMessage({ type: "sync", ...this.lastSync });
      if (this.pendingPreview !== null) {
        node.port.postMessage({ type: "preview", volume: this.pendingPreview });
        this.pendingPreview = null;
      }
      if (this.active && ctx.state === "suspended") {
        void ctx.resume().catch(() => {});
      }
      this.onStatusChange?.();
    } catch {
      this.unavailable = true;
      this.pendingPreview = null;
      this.onStatusChange?.();
    } finally {
      this.initializing = false;
    }
  }
}
