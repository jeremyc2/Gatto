# Rooftops audio and options

The soundtrack follows a warm nighttime groove: soft electric-key harmonics,
rounded bass, light brushed percussion, a neighborhood-specific lead and occasional
airy pads. The music supports the large apartment scenes and the cat's homeward
journey. Effects use short physical contacts and mechanisms, distinct from the
pitched soundtrack.

## Score

`src/rooftops/audio/synth.rs` renders stereo Float32 samples at 48 kHz. It uses
authored ninth/seventh chords, four eight-chord progressions and four melodic
motifs. A 64-bar arrangement moves through sparse introductions, answers,
register changes, breaks and bridges. Later cycles change the progression,
motif ordering and chord inversions. Each new playback session has its own seed
for arrangement and small performance variations. Rhythm uses a light swing.

The brownstone level uses soft keys at 88 BPM, the loft district rounded
mallets at 100 BPM, and the stone towers airy bells at 108 BPM. Later levels
raise the harmonic register, add rhythmic activity and introduce pads. Level
changes enter at a bar boundary, releasing sustained notes from the previous
key. Resting destinations use a softer, sparser arrangement. The final scorecard
has a separate 84 BPM celesta voice, a gentle ascending arrival figure and its
own progression.

Effects are limited to the sputtering intro thruster (following the visible
flame envelope), the cat's death, coin collection and apartment shutters
opening. The shutter cue follows closed-to-open apartment windows, including
intro touchdown. Relighting a rooftop checkpoint house during a level slide,
finishing or skipping that slide, and landing on already-open apartments are
silent. Jumping, landing impacts and enemy defeats are also silent.
Coins use measured high-frequency rings, band-limited contact noise and lower
surface taps, followed by uneven fading bounces. Shutter cues use a relay contact
followed by short mechanical flutter. Both stay separate from the score's key.

The coin patch uses metallic modes near 7.3, 9.8 and 12.1 kHz, filtered contact
noise and softer lower taps. Six fading contacts finish within 445 ms, with
small per-level pitch changes. The supplied recording informed the synthesis;
no recording, decoder or analysis data is needed during playback.
Both coin layers receive about 3 dB more gain to keep pickups audible relative
to the other effects without changing the shared effects-volume setting.

### Death reference

`audio/death.rs` renders the actual `playDeath` patch from the user's local
Visual Effect checkout, revision `2f910e4a2fe2910ed6f6f550e7531ca5afe73e94`,
[TaskSounds.ts](https://github.com/kitlangton/visual-effect/blob/2f910e4a2fe2910ed6f6f550e7531ca5afe73e94/src/sounds/TaskSounds.ts).
This replaces the previously selected falling reset cue. It keeps the same
patch in every neighborhood:

- D♯3 at velocity 0.45, released after 62.5 ms; C1 at velocity 0.55 starts 100 ms
  later and is held for 2 seconds (the reference's default 120 BPM transport).
- Three detuned saw oscillators over a 20-cent spread, each with ten harmonics
  and -9.3 dB gain. A normalized 4096-sample periodic table is prepared on the
  audio owner thread, before rendering starts.
- Attack 10 ms, decay 500 ms, sustain 0.3 and release 1.5 s. Exponential decay
  and release use the reference's approach curves and final linear tenth.
- Fully wet distortion 0.8, then a -12 dB output bus, with no reverb. Death
  bypasses the soundtrack's soft limiter to preserve the patch's dynamics.

The implementation follows the pinned Tone.js 15.1.22
[oscillator](https://github.com/Tonejs/Tone.js/blob/15.1.22/Tone/source/oscillator/FatOscillator.ts),
[envelope](https://github.com/Tonejs/Tone.js/blob/15.1.22/Tone/component/envelope/Envelope.ts),
[parameter curve](https://github.com/Tonejs/Tone.js/blob/15.1.22/Tone/core/context/Param.ts)
and [distortion](https://github.com/Tonejs/Tone.js/blob/15.1.22/Tone/effect/Distortion.ts)
behavior directly in Rust. Native sample interpolation and the app's effect
volume are separate from the browser's rendering, so this is a matching patch,
not a claim of identical recorded samples. No Tone.js runtime or Rust synthesis
dependency is added. MIT notices for Visual Effect and Tone.js are retained in
[packaging/licenses](../../packaging/licenses/) and copied into release bundles.

Fixed storage for 64 score/impact voices, eight death voices, a waveform table
and a preallocated stereo feedback delay keep rendering bounded. Music, effects
and master gains fade independently. A fully suspended score stops advancing
its sample clock. Volumes do not affect score/replay timing.

## Native output

`audio/output.rs` supplies samples through macOS AVAudioSourceNode and
AVAudioEngine, using the existing Objective-C binding family plus a direct
CoreAudioTypes binding. The source has an explicit planar Float32 stereo format;
the engine's mixer handles the output device format. The render block validates
buffer count, channel count and writable byte lengths before creating slices.
It uses a nonblocking try-lock and performs no allocation, file access, logging
or UI work. Playback errors leave the game playable and appear in Options.

The API's threading and buffer contracts come from Apple's
[source-node initializer](https://developer.apple.com/documentation/avfaudio/avaudiosourcenode/init(renderblock:))
and the binding's [render-block contract](https://docs.rs/objc2-avf-audio/0.3.2/objc2_avf_audio/type.AVAudioSourceNodeRenderBlock.html).
The UI changes only atomic controls. A dedicated owner thread creates, resumes,
suspends and releases the native graph. It sleeps while idle and polls a running
graph every 500 ms. Failures appear in Options; retries are bounded to a fresh
activation or volume change. Node shutdown stops and detaches the source before
releasing its render block. Closing the game releases its output graph without
waiting on the drawing thread.
Screenshots and ordinary automated tests never start device playback.

## Options and persistence

The top-right gear opens a dark card titled “Rooftops,” with Music, Sound effects,
Ghost cat and Animations controls, plus Continue. Music and Sound effects are
separate 0–100% sliders (0 is off), defaulting to 75% and 55%. Arrow keys adjust
by one point, Shift+arrow by five, and Home/End select the endpoints. Labels do
not use “pause” or “menu.” Outer form borders are absent; keyboard focus changes
label color, while slider handles retain their hover/drag highlights.
Opening it clears held input, jump buffering and frame catch-up, and
suspends game physics, scenery, transitions, replay time and audio. Escape
dismisses the card before the ordinary game-window close action. Keyboard focus
stays in the card while it is open. Holding the Music handle or pressing its
track auditions only that bus; the hit area includes the handle overhang at the
endpoints and excludes the row label and outer padding. Keyboard volume
adjustments also audition. Physics, transitions and the race remain held.
Release (including outside the card), Escape, Continue and loss of window focus
end the music audition. Sound effects stays silent while being adjusted, then
plays a single 150 ms sine beep on the final release only if the final volume
differs from the initial value. This also works outside the card and after
keyboard focus moves. Unchanged clicks, labels, padding and a zero setting are
silent.
The beep has its own fixed sample counter and envelope; it does not resume the
score or gameplay. It wakes the existing audio owner only as needed, without
UI timers or allocations in the callback. Dismissal and deactivation cancel it.
The retained-window close handler clears audition/adjustment state and restores
the game focus handle before hiding. Without that focus reset, reopening after
Options left input and the intro waiting on an unmounted control.

`Preferences` is an additive, defaulted field in the existing CBOR game data.
Preference writes run in the background. Slider movement reserves atomic
revisions, debounces for 250 ms, and flushes on release or Continue. Reserving a
revision uses a short state lock separate from disk I/O. Older queued choices
cannot undo newer ones, and saves queued before Reset Rooftops cannot restore
deleted data. Preference and completion saves read the latest file under the
same background I/O lock and atomic replacement path, preserving each other's
fields. Older records without preferences get defaults; earlier boolean mute
settings migrate to zero volume.

See the [game guide](../EASTER_EGG.md#options-and-sound) for user-facing behavior
and the [contributor guide](../../CONTRIBUTING.md#walkthrough-screenshots) for
rendering the card, exporting inspection WAVs and checking native output.
