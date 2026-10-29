//! A sample-clock score: authored harmony and motifs, arranged over 64 bars.
//! Rendering uses fixed voice storage and a preallocated stereo delay, with no
//! allocation, filesystem work or blocking operations on the audio thread.

use std::f32::consts::TAU;

pub(super) const SAMPLE_RATE: f32 = 48_000.;
const VOICES: usize = 64;
const CHORDS: [[u8; 5]; 8] = [
    [57, 60, 64, 67, 71], // Am9
    [53, 57, 60, 64, 67], // Fmaj9
    [60, 64, 67, 71, 74], // Cmaj9
    [55, 59, 62, 65, 69], // G9
    [50, 53, 57, 60, 64], // Dm9
    [52, 55, 59, 62, 66], // Em9
    [53, 57, 60, 64, 69], // Fmaj7/A
    [55, 59, 64, 69, 74], // G6/9
];
const PROGRESSIONS: [[usize; 8]; 4] = [
    [0, 1, 2, 3, 4, 0, 6, 7],
    [0, 4, 1, 3, 2, 5, 6, 7],
    [2, 6, 0, 4, 1, 5, 4, 7],
    [0, 6, 4, 7, 2, 1, 5, 3],
];
const MOTIFS: [[i8; 16]; 4] = [
    [0, -1, -1, 2, -1, -1, 3, -1, 4, -1, 3, -1, 2, -1, -1, -1],
    [3, -1, -1, -1, 2, -1, 0, -1, 1, -1, -1, 2, -1, -1, 0, -1],
    [-1, -1, 2, -1, 4, -1, -1, 3, -1, -1, 2, -1, 0, -1, -1, -1],
    [4, -1, 3, -1, -1, -1, 2, -1, 1, -1, -1, -1, 0, -1, -1, 2],
];
const SCALE: [u8; 7] = [0, 3, 5, 7, 10, 12, 15];

#[derive(Clone, Copy)]
pub(in crate::rooftops) enum Cue {
    Coin,
    Return,
    Shutters,
}

impl Cue {
    pub(super) fn bit(self) -> u32 {
        1 << self as u32
    }
}

#[derive(Clone, Copy, Default)]
enum Instrument {
    #[default]
    Keys,
    Bass,
    Pad,
    Kick,
    Brush,
    Hat,
    Mallet,
    Bell,
    Celesta,
    Coin,
    CoinBody,
    Click,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    instrument: Instrument,
    phase: f32,
    frequency: f32,
    age: f32,
    duration: f32,
    amplitude: f32,
    pan: f32,
    effect: bool,
    noise_filter: [f32; 2],
}

/// A fixed, allocation-free options audition. It never advances the score.
#[derive(Default)]
struct Preview {
    remaining: u32,
    phase: f32,
    gain: f32,
}

impl Preview {
    fn start(&mut self, volume: u8) {
        self.remaining = if volume == 0 { 0 } else { 7200 };
        self.phase = 0.;
        self.gain = (f32::from(volume.min(100)) / 100.).powi(2);
    }
    fn next(&mut self) -> f32 {
        if self.remaining == 0 {
            return 0.;
        }
        let attack = ((7200 - self.remaining) as f32 / 192.).min(1.);
        let release = (self.remaining as f32 / 1200.).min(1.);
        let sample = (self.phase * TAU).sin() * 0.16 * self.gain * attack * release;
        self.phase = (self.phase + 660. / SAMPLE_RATE).fract();
        self.remaining -= 1;
        sample
    }
}

pub(super) struct Synth {
    voices: [Voice; VOICES],
    delay: Vec<[f32; 2]>,
    delay_cursor: usize,
    random: u64,
    seed: usize,
    pub(super) level: usize,
    target_level: usize,
    resting: bool,
    bar: usize,
    step: usize,
    until_step: f64,
    samples: u64,
    master: f32,
    music_gain: f32,
    effects_gain: f32,
    rocket: bool,
    rocket_gain: f32,
    rocket_noise: f32,
    rocket_phase: f32,
    finale_entry: bool,
    death: super::death::Death,
    preview: Preview,
}

impl Synth {
    pub(super) fn new(seed: u64) -> Self {
        Self {
            voices: [Voice::default(); VOICES],
            delay: vec![[0.; 2]; (SAMPLE_RATE * 0.37) as usize],
            delay_cursor: 0,
            random: seed.max(1),
            seed: (seed % 97) as usize,
            level: 0,
            target_level: 0,
            resting: false,
            bar: 0,
            step: 0,
            until_step: 0.,
            samples: 0,
            master: 0.,
            music_gain: 0.,
            effects_gain: 0.,
            rocket: false,
            rocket_gain: 0.,
            rocket_noise: 0.,
            rocket_phase: 0.,
            finale_entry: false,
            death: super::death::Death::new(),
            preview: Preview::default(),
        }
    }

    pub(super) fn scene(&mut self, level: usize, resting: bool) {
        let level = level.min(3);
        if level == 3 && self.target_level != 3 {
            self.finale_entry = true;
        }
        self.target_level = level;
        self.resting = resting;
    }

    pub(super) fn thruster(&mut self, enabled: bool) {
        self.rocket = enabled;
    }

    fn random(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random >> 40) as f32 / 16_777_216.
    }

    fn note(
        &mut self,
        instrument: Instrument,
        midi: f32,
        duration: f32,
        amplitude: f32,
        placement: (f32, f32),
        effect: bool,
    ) {
        let (pan, delay) = placement;
        let index = self
            .voices
            .iter()
            .position(|voice| voice.duration <= voice.age)
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| (a.duration - a.age).total_cmp(&(b.duration - b.age)))
                    .map_or(0, |(index, _)| index)
            });
        self.voices[index] = Voice {
            instrument,
            phase: 0.,
            frequency: 440. * 2_f32.powf((midi - 69.) / 12.),
            age: -delay,
            duration,
            amplitude,
            pan: pan.clamp(-1., 1.),
            effect,
            noise_filter: [0.; 2],
        };
    }

    fn schedule(&mut self) {
        if self.step == 0 {
            if self.level != self.target_level {
                for voice in &mut self.voices {
                    if !voice.effect && voice.age < voice.duration {
                        voice.duration = voice.duration.min(voice.age.max(0.) + 0.08);
                    }
                }
            }
            self.level = self.target_level;
        }
        let beat = 60. / [88., 100., 108., 84.][self.level];
        let transpose = [0, 3, 5, 0][self.level];
        let cycle = self.bar / 64;
        let section = (self.bar / 8) % 8;
        let progression = if self.level == 3 {
            2
        } else {
            (cycle + self.level + self.seed) % PROGRESSIONS.len()
        };
        if self.finale_entry && self.step == 0 {
            self.finale_entry = false;
            for (index, note) in [72., 76., 79., 83., 86.].into_iter().enumerate() {
                self.note(
                    Instrument::Celesta,
                    note,
                    1.8,
                    0.035,
                    (index as f32 * 0.15 - 0.3, index as f32 * 0.13),
                    false,
                );
            }
        }
        let chord = CHORDS[PROGRESSIONS[progression][(self.bar / 2) % 8]];
        let sparse = section == 0 || section == 3 || self.resting;
        if self.step == 0 || (!sparse && self.step == 10) {
            let inversion = (self.bar / 8 + cycle) % 3;
            for (index, note) in chord.into_iter().enumerate() {
                let octave = if index < inversion { 12 } else { 0 };
                let velocity = 0.024 * (0.9 + self.random() * 0.2);
                self.note(
                    Instrument::Keys,
                    (note + transpose + octave) as f32,
                    beat * if self.step == 0 { 3.4 } else { 1.4 },
                    velocity,
                    (index as f32 * 0.22 - 0.44, index as f32 * 0.009),
                    false,
                );
            }
        }
        if self.level >= 2 && self.step == 0 && self.bar.is_multiple_of(2) {
            for note in [chord[1], chord[3]] {
                self.note(
                    Instrument::Pad,
                    (note + transpose + 12) as f32,
                    beat * 7.,
                    0.012,
                    (0.3, 0.),
                    false,
                );
            }
        }
        if self.step == 0 || self.step == 8 || (!sparse && [6, 14].contains(&self.step)) {
            let note = if self.step == 6 {
                chord[2]
            } else if self.step == 14 {
                chord[1]
            } else {
                chord[0]
            };
            self.note(
                Instrument::Bass,
                (note + transpose - 12) as f32,
                beat * 0.85,
                0.095,
                (-0.08, 0.),
                false,
            );
        }
        if (!sparse || (section == 0 && self.bar % 4 == 2) || self.level == 3)
            && section != 7
            && (!self.resting || self.level == 3)
        {
            let motif = (section + self.bar % 4 + cycle + self.seed) % MOTIFS.len();
            let degree = MOTIFS[motif][self.step];
            // Phrases breathe: an answer every other bar, plus a bridge register.
            if degree >= 0 && (self.bar % 4 != 3 || self.step < 8) {
                let octave = if section == 5 || self.level == 2 {
                    12
                } else {
                    0
                };
                let note = 69 + transpose + SCALE[degree as usize] + octave;
                let velocity = 0.034 * (0.8 + self.random() * 0.3);
                self.note(
                    [
                        Instrument::Keys,
                        Instrument::Mallet,
                        Instrument::Bell,
                        Instrument::Celesta,
                    ][self.level],
                    note as f32,
                    beat * 0.8,
                    velocity,
                    (
                        if self.bar.is_multiple_of(2) {
                            -0.28
                        } else {
                            0.28
                        },
                        0.,
                    ),
                    false,
                );
            }
        }
        if self.step == 0 || self.step == 8 {
            self.note(Instrument::Kick, 36., 0.22, 0.105, (0., 0.), false);
        }
        if self.step == 4 || self.step == 12 {
            self.note(
                Instrument::Brush,
                52.,
                0.16,
                if sparse { 0.025 } else { 0.045 },
                (0.18, 0.),
                false,
            );
        }
        if self.step % 4 == 2 || (!sparse && self.level > 0 && self.step.is_multiple_of(4)) {
            let velocity = 0.018 * (0.7 + self.random() * 0.5);
            self.note(Instrument::Hat, 90., 0.055, velocity, (-0.35, 0.), false);
        }
        // Occasional phrase-ending fills, never a fill on every measure.
        if !sparse && self.bar % 8 == 7 && [13, 15].contains(&self.step) {
            self.note(Instrument::Brush, 57., 0.11, 0.025, (-0.2, 0.), false);
        }
        let swing = if self.step % 4 < 2 { 0.28 } else { 0.22 };
        self.until_step += (beat * swing * SAMPLE_RATE) as f64;
        self.step += 1;
        if self.step == 16 {
            self.step = 0;
            self.bar += 1;
        }
    }

    pub(super) fn preview_effects(&mut self, volume: u8) {
        self.preview.start(volume);
    }
    pub(super) fn previewing(&self) -> bool {
        self.preview.remaining > 0
    }
    pub(super) fn preview_sample(&mut self) -> f32 {
        self.preview.next()
    }

    pub(super) fn cues(&mut self, bits: u32) {
        if bits & Cue::Coin.bit() != 0 {
            // Reference: metal rings near 7.3 / 9.8 / 12.1 kHz over contact taps,
            // followed by uneven, fading bounces. Trim the quiet leading 0.11 s.
            let center = [9800., 9950., 9650., 9800.][self.target_level];
            for (delay, strength) in [
                (0., 1.),
                (0.09, 0.47),
                (0.14, 0.45),
                (0.21, 0.18),
                (0.265, 0.24),
                (0.35, 0.07),
            ] {
                let metal_hz = center * (0.996 + self.random() * 0.008);
                let metal_note = 69. + 12. * (metal_hz / 440.).log2();
                self.note(
                    Instrument::Coin,
                    metal_note,
                    0.095,
                    0.12 * strength,
                    (0.03, delay),
                    true,
                );
                let body_note = 69. + 12. * (565_f32 / 440.).log2();
                self.note(
                    Instrument::CoinBody,
                    body_note,
                    0.045,
                    0.078 * strength,
                    (0., delay),
                    true,
                );
            }
        }
        if bits & Cue::Return.bit() != 0 {
            self.death.trigger();
        }
        if bits & Cue::Shutters.bit() != 0 {
            // Relay contact, then a short shutter mechanism flutter.
            let note = [57., 61., 54., 57.][self.target_level];
            for (delay, amplitude) in [(0., 0.13), (0.024, 0.075), (0.052, 0.045)] {
                self.note(Instrument::Click, note, 0.065, amplitude, (0., delay), true);
            }
        }
    }

    pub(super) fn next(&mut self, active: bool, music: u8, effects: u8) -> [f32; 2] {
        self.master += ((active as u8 as f32) - self.master) * 0.003;
        if !active && self.master < 0.0001 {
            self.master = 0.;
            return [0.; 2];
        }
        let music_target =
            (f32::from(music.min(100)) / 100.).powi(2) * if self.resting { 0.52 } else { 1. };
        let effects_target = (f32::from(effects.min(100)) / 100.).powi(2);
        self.music_gain += (music_target - self.music_gain) * 0.0015;
        self.effects_gain += (effects_target - self.effects_gain) * 0.003;
        if self.until_step <= 0. {
            self.schedule();
        }
        self.until_step -= 1.;
        self.samples += 1;
        let noise = self.random() * 2. - 1.;
        let mut music_bus = [0_f32; 2];
        let mut effects_bus = [0_f32; 2];
        for voice in &mut self.voices {
            if voice.age >= voice.duration {
                continue;
            }
            voice.age += 1. / SAMPLE_RATE;
            if voice.age < 0. {
                continue;
            }
            let age = voice.age;
            let frequency = match voice.instrument {
                Instrument::Kick => 46. + 92. * (-age * 32.).exp(),
                _ => voice.frequency,
            };
            voice.phase = (voice.phase + frequency / SAMPLE_RATE).fract();
            let phase = voice.phase * TAU;
            let attack = (age
                * if matches!(
                    voice.instrument,
                    Instrument::Coin | Instrument::Click | Instrument::CoinBody
                ) {
                    2400.
                } else {
                    160.
                })
            .min(1.);
            let release = ((voice.duration - age) * 30.).clamp(0., 1.);
            let wave = match voice.instrument {
                Instrument::Keys => {
                    (phase.sin()
                        + (phase * 2.).sin() * 0.25 * (-age * 6.).exp()
                        + (phase * 3.).sin() * 0.1 * (-age * 10.).exp())
                        * (-age * 2.5).exp()
                }
                Instrument::Bass => (phase.sin() + (phase * 2.).sin() * 0.17) * (-age * 2.8).exp(),
                Instrument::Pad => {
                    (phase.sin() * 0.65
                        + (phase + age * voice.frequency * 0.002 * TAU).sin() * 0.35)
                        * (age * 2.).min(1.)
                        * ((voice.duration - age) * 1.4).clamp(0., 1.)
                }
                Instrument::Kick => phase.sin() * (-age * 21.).exp(),
                Instrument::Brush => (noise * 0.8 + phase.sin() * 0.2) * (-age * 28.).exp(),
                Instrument::Hat => noise * (-age * 100.).exp(),
                Instrument::Mallet => {
                    (phase.sin() + (phase * 3.).sin() * 0.28 * (-age * 16.).exp())
                        * (-age * 11.).exp()
                }
                Instrument::Bell => {
                    (phase.sin()
                        + (age * voice.frequency * 2.76 * TAU).sin() * 0.22 * (-age * 8.).exp())
                        * (-age * 4.2).exp()
                }
                Instrument::Celesta => {
                    (phase.sin()
                        + (phase * 2.).sin() * 0.18
                        + (phase * 4.).sin() * 0.08 * (-age * 6.).exp())
                        * (-age * 3.8).exp()
                }
                Instrument::Coin => {
                    let strike = age * voice.frequency * TAU;
                    // High-band contact noise excites the metal. These two
                    // filter states are fixed per voice, never allocated here.
                    voice.noise_filter[0] += (noise - voice.noise_filter[0]) * 0.18;
                    let high = noise - voice.noise_filter[0];
                    voice.noise_filter[1] += (high - voice.noise_filter[1]) * 0.78;
                    strike.sin() * 0.45 * (-age * 65.).exp()
                        + (strike * (7300. / 9800.)).sin() * 0.34 * (-age * 85.).exp()
                        + (strike * (12100. / 9800.)).sin() * 0.20 * (-age * 110.).exp()
                        + (strike * 0.993).sin() * 0.12 * (-age * 75.).exp()
                        + (strike * 1.007).sin() * 0.10 * (-age * 90.).exp()
                        + voice.noise_filter[1] * 0.45 * (-age * 60.).exp()
                }
                Instrument::CoinBody => {
                    let contact = age * voice.frequency * TAU;
                    voice.noise_filter[0] += (noise - voice.noise_filter[0]) * 0.14;
                    contact.sin() * 0.6 * (-age * 140.).exp()
                        + (contact * 1.604).sin() * 0.25 * (-age * 180.).exp()
                        + (contact * 1.9).sin() * 0.18 * (-age * 220.).exp()
                        + voice.noise_filter[0] * 0.4 * (-age * 180.).exp()
                }
                Instrument::Click => {
                    (noise * 0.6 + phase.sin() * 0.4)
                        * (-age * [110., 135., 90., 110.][self.level]).exp()
                }
            };
            let sample = wave * attack * release * voice.amplitude;
            let bus = if voice.effect {
                &mut effects_bus
            } else {
                &mut music_bus
            };
            bus[0] += sample * (1. - voice.pan * 0.5);
            bus[1] += sample * (1. + voice.pan * 0.5);
        }
        self.rocket_gain += ((self.rocket as u8 as f32) - self.rocket_gain) * 0.004;
        self.rocket_noise += (noise - self.rocket_noise) * 0.09;
        self.rocket_phase = (self.rocket_phase + 74. / SAMPLE_RATE).fract();
        let rocket =
            ((self.rocket_phase * TAU).sin() * 0.032 + self.rocket_noise * 0.11) * self.rocket_gain;
        effects_bus[0] += rocket;
        effects_bus[1] += rocket * 0.94;
        let delayed = self.delay[self.delay_cursor];
        self.delay[self.delay_cursor] = [
            music_bus[0] + delayed[1] * 0.24,
            music_bus[1] + delayed[0] * 0.24,
        ];
        self.delay_cursor = (self.delay_cursor + 1) % self.delay.len();
        let death = self.death.next() * self.effects_gain * self.master;
        std::array::from_fn(|channel| {
            let mix = ((music_bus[channel] + delayed[channel] * 0.17) * self.music_gain
                + effects_bus[channel] * self.effects_gain)
                * self.master;
            // Soft headroom for overlapping notes and effects, below full scale.
            let mix = mix * 1.8;
            // Preserve the reference death patch's distortion and dynamics;
            // it has its own -12 dB output gain, outside the music soft limiter.
            (mix / (1. + mix.abs()) * 0.85 + death).clamp(-0.95, 0.95)
        })
    }
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
pub(super) fn audit(directory: &std::path::Path) -> anyhow::Result<()> {
    use std::time::Instant;
    let mut reports = Vec::new();
    for level in 0..4 {
        let mut synth = Synth::new(0x676174746f);
        synth.scene(level, level == 3);
        let blocks = 48_000 * 180 / 512;
        let mut timings = Vec::with_capacity(blocks);
        let mut peak = 0_f32;
        let mut energy = 0_f64;
        for block in 0..blocks {
            let start = Instant::now();
            synth.thruster(level == 0 && block < 94);
            if block % 197 == 0 {
                synth.cues(7);
            }
            for _ in 0..512 {
                let samples = std::hint::black_box(synth.next(true, 100, 100));
                for sample in samples {
                    anyhow::ensure!(sample.is_finite(), "Non-finite audio sample");
                    peak = peak.max(sample.abs());
                    energy += f64::from(sample).powi(2);
                }
            }
            timings.push(start.elapsed().as_secs_f64() * 1000.);
        }
        timings.sort_by(f64::total_cmp);
        let report = serde_json::json!({
            "scene": level, "seconds": 180, "block_frames": 512, "budget_ms": 512. / 48.,
            "p50_ms": timings[blocks / 2], "p99_ms": timings[blocks * 99 / 100],
            "max_ms": timings[blocks - 1], "peak": peak,
            "rms": (energy / (blocks * 1024) as f64).sqrt(),
            "synth_bytes": std::mem::size_of::<Synth>() + synth.delay.capacity() * std::mem::size_of::<[f32; 2]>() + 4096 * std::mem::size_of::<f32>(),
        });
        println!("{report}");
        reports.push(report);
    }
    std::fs::write(
        directory.join("rooftops-audio.json"),
        serde_json::to_vec_pretty(&reports)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_and_all_effects_stay_finite_bounded_and_stereo_through_level_changes() {
        let mut synth = Synth::new(42);
        let mut energy = [0_f64; 2];
        let mut peak = 0_f32;
        let mut difference = 0_f64;
        for level in 0..4 {
            synth.scene(level, false);
            synth.cues(7);
            for _ in 0..48_000 * 8 {
                let frame = synth.next(true, 100, 100);
                for channel in 0..2 {
                    assert!(frame[channel].is_finite());
                    peak = peak.max(frame[channel].abs());
                    energy[channel] += f64::from(frame[channel]).powi(2);
                }
                difference += f64::from(frame[0] - frame[1]).powi(2);
            }
            assert_eq!(synth.level, level);
        }
        assert!(peak < 0.85 && peak > 0.05);
        assert!(energy.iter().all(|value| *value > 1.));
        assert!(difference > 1.);
    }

    #[test]
    fn coin_has_the_reference_high_rings_and_a_short_fading_tail() {
        let mut synth = Synth::new(0x676174746f);
        for _ in 0..4000 {
            synth.next(true, 0, 100);
        }
        synth.cues(Cue::Coin.bit());
        let samples: Vec<_> = (0..48_000).map(|_| synth.next(true, 0, 100)[0]).collect();
        assert!(
            samples[24_000..].iter().all(|sample| *sample == 0.),
            "coin finishes within half a second"
        );
        let window: Vec<_> = samples[960..3360]
            .iter()
            .enumerate()
            .map(|(i, sample)| {
                f64::from(*sample) * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / 2399.).cos())
            })
            .collect();
        // Goertzel band peaks inspect generated output; they do not depend on
        // which oscillators or filters the implementation uses.
        let band = |center: f64| {
            (-6..=6)
                .map(|offset| {
                    let coefficient = 2.
                        * (std::f64::consts::TAU * (center + f64::from(offset) * 20.) / 48_000.)
                            .cos();
                    let (mut previous, mut older) = (0., 0.);
                    for sample in &window {
                        let value = sample + coefficient * previous - older;
                        older = previous;
                        previous = value;
                    }
                    previous * previous + older * older - coefficient * previous * older
                })
                .fold(0_f64, f64::max)
        };
        let background = band(3000.);
        assert!(band(9800.) > background * 3.);
        assert!(band(7300.) > background * 2.);
        assert!(band(12100.) > background);
    }

    #[test]
    fn effects_preview_has_selected_gain_finishes_and_does_not_advance_music() {
        let mut loud = Synth::new(7);
        let mut quiet = Synth::new(7);
        loud.preview_effects(100);
        quiet.preview_effects(50);
        let mut peak = 0_f32;
        for _ in 0..7200 {
            let sample = loud.preview_sample();
            peak = peak.max(sample.abs());
            assert!((quiet.preview_sample() - sample * 0.25).abs() < 0.00001);
        }
        assert!(peak > 0.15 && peak <= 0.16);
        assert!(!loud.previewing());
        assert_eq!(loud.preview_sample(), 0.);
        assert_eq!(loud.samples, 0);
        loud.preview_effects(0);
        assert!(!loud.previewing());
        assert_eq!(loud.preview_sample(), 0.);
    }

    #[test]
    fn suspension_fades_to_silence_and_freezes_the_musical_clock() {
        let mut synth = Synth::new(7);
        for _ in 0..48_000 {
            synth.next(true, 100, 100);
        }
        for _ in 0..5_000 {
            synth.next(false, 100, 100);
        }
        let samples = synth.samples;
        for _ in 0..10_000 {
            assert_eq!(synth.next(false, 100, 100), [0.; 2]);
        }
        assert_eq!(synth.samples, samples);
        synth.next(true, 100, 100);
        assert_eq!(synth.samples, samples + 1);
    }

    #[test]
    fn music_and_effects_can_be_muted_independently() {
        let mut synth = Synth::new(7);
        for _ in 0..8_000 {
            assert_eq!(synth.next(true, 0, 0), [0.; 2]);
        }
        synth.cues(Cue::Coin.bit());
        assert!((0..4_000).any(|_| {
            synth
                .next(true, 0, 100)
                .iter()
                .any(|sample| sample.abs() > 0.001)
        }));
        for _ in 0..48_000 {
            synth.next(true, 0, 0);
        }
        assert!(
            synth
                .next(true, 0, 0)
                .iter()
                .all(|sample| sample.abs() < 0.0001)
        );
        assert!((0..4_000).any(|_| {
            synth
                .next(true, 100, 0)
                .iter()
                .any(|sample| sample.abs() > 0.001)
        }));
    }
}
