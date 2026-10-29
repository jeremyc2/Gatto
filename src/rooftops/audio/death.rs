//! Native rendering of Visual Effect's playDeath patch. See the accompanying
//! source/license record in docs/design/rooftops-audio.md and packaging notices.
//! All tables and voices are allocated before the audio callback starts.

use super::synth::SAMPLE_RATE;

const TABLE: usize = 4096;
const RELEASE: f32 = 1.5;

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    frame: i32,
    frequency: f32,
    gate: f32,
    velocity: f32,
    phases: [f32; 3],
}

pub(super) struct Death {
    voices: [Voice; 8],
    saw: Box<[f32; TABLE]>,
}

impl Death {
    pub(super) fn new() -> Self {
        // fatsawtooth10: ten alternating 1/n harmonics, normalized like a
        // Web Audio periodic wave. Three copies are spread over twenty cents.
        let mut saw = Box::new([0_f32; TABLE]);
        let mut peak = 0_f32;
        for (index, sample) in saw.iter_mut().enumerate() {
            let phase = index as f32 / TABLE as f32 * std::f32::consts::TAU;
            *sample = (1..=10)
                .map(|n| (phase * n as f32).sin() * if n % 2 == 0 { -1. } else { 1. } / n as f32)
                .sum();
            peak = peak.max(sample.abs());
        }
        for sample in saw.iter_mut() {
            *sample /= peak;
        }
        Self {
            voices: [Voice::default(); 8],
            saw,
        }
    }

    pub(super) fn trigger(&mut self) {
        // Default Tone transport is 120 BPM: 32n = 62.5 ms, 1n = 2 s.
        for (midi, gate, delay, velocity) in [(51., 0.0625, 0., 0.45), (24., 2., 0.1, 0.55)] {
            let index = self
                .voices
                .iter()
                .position(|voice| !voice.active)
                .unwrap_or_else(|| {
                    self.voices
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, voice)| voice.frame)
                        .unwrap()
                        .0
                });
            self.voices[index] = Voice {
                active: true,
                frame: -(delay * SAMPLE_RATE) as i32,
                frequency: 440. * 2_f32.powf((midi - 69.) / 12.),
                gate,
                velocity,
                // Tone's oscillator phase rotates its Fourier components.
                phases: [0., 2. / 3., 1. / 3.],
            };
        }
    }

    pub(super) fn next(&mut self) -> f32 {
        let mut sum = 0.;
        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }
            let age = voice.frame as f32 / SAMPLE_RATE;
            voice.frame += 1;
            if age < 0. {
                continue;
            }
            if age >= voice.gate + RELEASE {
                voice.active = false;
                continue;
            }
            let envelope = envelope(age, voice.gate) * voice.velocity;
            for (phase, detune) in voice.phases.iter_mut().zip([0.9942404, 1., 1.0057929]) {
                let position = *phase * TABLE as f32;
                let index = position as usize % TABLE;
                let wave = self.saw[index]
                    + (self.saw[(index + 1) % TABLE] - self.saw[index]) * position.fract();
                // FatOscillator's per-oscillator gain: -6 - count * 1.1 dB.
                sum += wave * envelope * 0.34276778;
                *phase = (*phase + voice.frequency * detune / SAMPLE_RATE).fract();
            }
        }
        // Fully wet Tone distortion 0.8, then the reference's -12 dB bus.
        if sum.abs() < 0.001 {
            0.
        } else {
            ((83. * sum * 20. * std::f32::consts::PI / 180.)
                / (std::f32::consts::PI + 80. * sum.abs()))
                * 0.25118864
        }
    }
}

// Tone's exponential approach switches to a linear finish for the last tenth.
fn approach(from: f32, target: f32, time: f32, duration: f32) -> f32 {
    let at = |t: f32| target + (from - target) * (-t / ((duration + 1.).ln() / 200_f32.ln())).exp();
    if time >= duration {
        target
    } else if time < duration * 0.9 {
        at(time)
    } else {
        target + (at(duration * 0.9) - target) * ((duration - time) / (duration * 0.1))
    }
}

fn envelope(age: f32, gate: f32) -> f32 {
    let held = |time: f32| {
        if time < 0.01 {
            time / 0.01
        } else {
            approach(1., 0.3, time - 0.01, 0.5)
        }
    };
    if age < gate {
        held(age)
    } else {
        approach(held(gate), 0., age - gate, RELEASE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_patch_keeps_its_notes_offsets_gate_times_and_release() {
        let mut death = Death::new();
        death.trigger();
        assert!((death.voices[0].frequency - 155.56349).abs() < 0.001);
        assert!((death.voices[1].frequency - 32.703197).abs() < 0.001);
        assert_eq!(death.voices[1].frame, -4800);
        assert_eq!(death.voices[0].gate, 0.0625);
        assert_eq!(death.voices[1].gate, 2.);
        let mut tail = 0_f32;
        for frame in 0..(SAMPLE_RATE * 4.) as usize {
            let sample = death.next();
            assert!(sample.is_finite() && sample.abs() < 0.1);
            if (48_000..96_000).contains(&frame) {
                tail = tail.max(sample.abs());
            }
            if frame > 173_000 {
                assert_eq!(sample, 0.);
            }
        }
        assert!(tail > 0.01, "the selected death has a sustained low rumble");
    }
}
