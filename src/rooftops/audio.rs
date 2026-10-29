//! Atomic game controls and a dedicated native-audio owner thread.

mod death;
#[cfg(target_os = "macos")]
mod output;
mod synth;

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
    mpsc::{Receiver, SyncSender, sync_channel},
};
pub(super) use synth::Cue;

#[derive(Clone, Copy)]
pub(super) struct Scene {
    pub active: bool,
    pub level: usize,
    pub resting: bool,
    pub thruster: bool,
}

struct Controls {
    active: AtomicBool,
    music: AtomicU32,
    effects: AtomicU32,
    level: AtomicUsize,
    resting: AtomicBool,
    thruster: AtomicBool,
    cues: AtomicU32,
    // 0: no request, 1: cancel/mute, 2..101: selected volume + 1.
    preview: AtomicU32,
    previewing: AtomicBool,
    rendered_frames: AtomicU64,
    render_errors: AtomicU32,
    epoch: AtomicU64,
    retry: AtomicU64,
    shutdown: AtomicBool,
    unavailable: AtomicBool,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            active: AtomicBool::new(false),
            music: AtomicU32::new(75),
            effects: AtomicU32::new(55),
            level: AtomicUsize::new(0),
            resting: AtomicBool::new(false),
            thruster: AtomicBool::new(false),
            cues: AtomicU32::new(0),
            preview: AtomicU32::new(0),
            previewing: AtomicBool::new(false),
            rendered_frames: AtomicU64::new(0),
            render_errors: AtomicU32::new(0),
            epoch: AtomicU64::new(0),
            retry: AtomicU64::new(0),
            shutdown: AtomicBool::new(false),
            unavailable: AtomicBool::new(false),
        }
    }
}

pub(super) struct Audio {
    controls: Arc<Controls>,
    wake: Option<SyncSender<()>>,
    events: Option<async_channel::Receiver<()>>,
}

impl Audio {
    pub(super) fn new(enabled: bool) -> Self {
        let controls = Arc::new(Controls::default());
        let (wake, receiver) = sync_channel(1);
        let (events, updates) = async_channel::bounded(1);
        #[cfg(target_os = "macos")]
        if enabled {
            let capture = controls.clone();
            if let Err(error) = std::thread::Builder::new()
                .name("rooftops-audio".into())
                .spawn(move || worker(capture, receiver, events))
            {
                eprintln!("Could not start Rooftops audio: {error}");
                controls.unavailable.store(true, Ordering::Release);
            }
        }
        Self {
            controls,
            wake: enabled.then_some(wake),
            events: Some(updates),
        }
    }

    fn wake(&self) {
        if let Some(wake) = &self.wake {
            let _ = wake.try_send(());
        }
    }

    pub(super) fn take_events(&mut self) -> Option<async_channel::Receiver<()>> {
        self.events.take()
    }

    pub(super) fn sync(
        &self,
        scene: Scene,
        preferences: crate::rooftops_score::replay::Preferences,
    ) {
        self.controls
            .music
            .store(u32::from(preferences.music_volume), Ordering::Relaxed);
        self.controls
            .effects
            .store(u32::from(preferences.effects_volume), Ordering::Relaxed);
        self.controls.level.store(scene.level, Ordering::Relaxed);
        self.controls
            .resting
            .store(scene.resting, Ordering::Relaxed);
        self.controls
            .thruster
            .store(scene.thruster, Ordering::Relaxed);
        let active =
            scene.active && (preferences.music_volume > 0 || preferences.effects_volume > 0);
        if self.controls.active.swap(active, Ordering::AcqRel) != active {
            self.wake();
        }
    }

    pub(super) fn cue(&self, cue: Cue) {
        if self.controls.active.load(Ordering::Acquire)
            && self.controls.effects.load(Ordering::Relaxed) > 0
        {
            self.controls.cues.fetch_or(cue.bit(), Ordering::Relaxed);
        }
    }

    pub(super) fn preview_effects(&self, volume: u8) {
        if volume > 0 {
            self.controls.retry.fetch_add(1, Ordering::AcqRel);
        }
        self.controls
            .preview
            .store(u32::from(volume.min(100)) + 1, Ordering::Release);
        self.wake();
    }

    #[cfg(test)]
    pub(super) fn test_cues(&self) -> u32 {
        self.controls.cues.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(super) fn test_preview(&self) -> u32 {
        self.controls.preview.load(Ordering::Acquire)
    }

    pub(super) fn suspend(&self) {
        self.controls.active.store(false, Ordering::Release);
        self.controls.cues.store(0, Ordering::Relaxed);
        self.controls.preview.store(1, Ordering::Release);
        self.controls.previewing.store(false, Ordering::Release);
        self.wake();
    }

    pub(super) fn stop(&self) {
        self.suspend();
        self.controls.epoch.fetch_add(1, Ordering::AcqRel);
        self.wake();
    }
    pub(super) fn retry(&self) {
        self.controls.retry.fetch_add(1, Ordering::AcqRel);
        self.wake();
    }
    #[cfg(test)]
    pub(super) fn test_output(&self) -> (bool, u32, u32) {
        (
            self.controls.active.load(Ordering::Acquire),
            self.controls.music.load(Ordering::Relaxed),
            self.controls.effects.load(Ordering::Relaxed),
        )
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.controls
            .unavailable
            .load(Ordering::Acquire)
            .then_some("Audio is unavailable on this device.")
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.controls.active.store(false, Ordering::Release);
        self.controls.shutdown.store(true, Ordering::Release);
        self.wake();
    }
}

#[cfg(target_os = "macos")]
fn worker(controls: Arc<Controls>, receiver: Receiver<()>, events: async_channel::Sender<()>) {
    use std::time::{Duration, Instant};
    let mut output: Option<output::Output> = None;
    let mut epoch = controls.epoch.load(Ordering::Acquire);
    let mut retry = controls.retry.load(Ordering::Acquire);
    let mut attempted = false;
    let mut was_active = false;
    let mut paused = true;
    let mut last_active = Instant::now();
    loop {
        if controls.shutdown.load(Ordering::Acquire) {
            break;
        }
        let result = if was_active {
            receiver
                .recv_timeout(Duration::from_millis(
                    if controls.active.load(Ordering::Acquire) {
                        500
                    } else {
                        80
                    },
                ))
                .map_err(|error| matches!(error, std::sync::mpsc::RecvTimeoutError::Disconnected))
        } else if !paused {
            receiver
                .recv_timeout(Duration::from_millis(80))
                .map_err(|error| matches!(error, std::sync::mpsc::RecvTimeoutError::Disconnected))
        } else {
            receiver.recv().map_err(|_| true)
        };
        if result == Err(true) || controls.shutdown.load(Ordering::Acquire) {
            break;
        }
        objc2::rc::autoreleasepool(|_| {
            let current_epoch = controls.epoch.load(Ordering::Acquire);
            if current_epoch != epoch {
                output = None;
                epoch = current_epoch;
                attempted = false;
                paused = true;
            }
            let current_retry = controls.retry.load(Ordering::Acquire);
            if current_retry != retry {
                retry = current_retry;
                attempted = false;
            }
            let active = controls.active.load(Ordering::Acquire)
                || controls.preview.load(Ordering::Acquire) > 1
                || controls.previewing.load(Ordering::Acquire);
            if active && !was_active {
                attempted = false;
            }
            if active {
                if output.is_none() && !attempted {
                    attempted = true;
                    match output::Output::new(controls.clone()) {
                        Ok(created) => {
                            output = Some(created);
                            controls.unavailable.store(false, Ordering::Release);
                        }
                        Err(error) => {
                            eprintln!("Rooftops audio unavailable: {error:#}");
                            controls.unavailable.store(true, Ordering::Release);
                            controls.preview.store(0, Ordering::Release);
                            controls.previewing.store(false, Ordering::Release);
                        }
                    }
                    let _ = events.try_send(());
                } else if let Some(current) = &output
                    && let Err(error) = current.ensure_running()
                {
                    eprintln!("Could not resume Rooftops audio: {error:#}");
                    output = None;
                    attempted = true;
                    controls.unavailable.store(true, Ordering::Release);
                    controls.preview.store(0, Ordering::Release);
                    controls.previewing.store(false, Ordering::Release);
                    let _ = events.try_send(());
                }
                last_active = Instant::now();
                paused = false;
            } else if !paused && last_active.elapsed() >= Duration::from_millis(80) {
                if let Some(output) = &output {
                    output.pause();
                }
                paused = true;
            }
            was_active = active;
        });
    }
    // Native shutdown stays on this owner thread, never on the frame thread.
    drop(output);
}

#[cfg(all(feature = "walkthrough-screenshots", target_os = "macos"))]
#[allow(dead_code)]
pub(crate) fn check_output() -> anyhow::Result<()> {
    objc2::rc::autoreleasepool(|_| -> anyhow::Result<()> {
        let controls = Arc::new(Controls::default());
        let output = output::Output::new(controls.clone())?;
        std::thread::sleep(std::time::Duration::from_millis(300));
        output.pause();
        let paused = controls.rendered_frames.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(100));
        anyhow::ensure!(
            controls.rendered_frames.load(Ordering::Relaxed) == paused,
            "Native output kept rendering after suspension"
        );
        output.ensure_running()?;
        std::thread::sleep(std::time::Duration::from_millis(150));
        drop(output);
        anyhow::ensure!(
            controls.rendered_frames.load(Ordering::Relaxed) > paused,
            "Native output did not resume"
        );
        anyhow::ensure!(
            controls.render_errors.load(Ordering::Relaxed) == 0,
            "Native audio buffer contract failed"
        );
        println!(
            "Native stereo output rendered {} silent frames, suspended and resumed without buffer errors.",
            controls.rendered_frames.load(Ordering::Relaxed)
        );
        Ok(())
    })?;
    let audio = Audio::new(true);
    audio.controls.music.store(0, Ordering::Relaxed);
    audio.controls.effects.store(0, Ordering::Relaxed);
    // Exercise the real owner with a synthesizing but inaudible graph.
    audio.controls.active.store(true, Ordering::Release);
    audio.wake();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while audio.controls.rendered_frames.load(Ordering::Relaxed) == 0
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    anyhow::ensure!(
        audio.controls.rendered_frames.load(Ordering::Relaxed) > 0,
        "Audio owner did not start"
    );
    audio.suspend();
    std::thread::sleep(std::time::Duration::from_millis(250));
    let paused = audio.controls.rendered_frames.load(Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    anyhow::ensure!(
        audio.controls.rendered_frames.load(Ordering::Relaxed) == paused,
        "Owner did not suspend"
    );
    // 1% produces a ~-96 dBFS test signal: exercise native one-shot playback
    // without an audible beep or advancing the suspended background score.
    audio.preview_effects(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while audio.controls.rendered_frames.load(Ordering::Relaxed) < paused + 7200
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    anyhow::ensure!(
        audio.controls.rendered_frames.load(Ordering::Relaxed) >= paused + 7200,
        "Effect preview did not wake paused output"
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    anyhow::ensure!(
        !audio.controls.active.load(Ordering::Acquire)
            && !audio.controls.previewing.load(Ordering::Acquire),
        "Effect preview did not finish while the score stayed suspended"
    );
    let preview_finished = audio.controls.rendered_frames.load(Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    anyhow::ensure!(
        audio.controls.rendered_frames.load(Ordering::Relaxed) == preview_finished,
        "Effect preview left silent output running"
    );
    audio.controls.active.store(true, Ordering::Release);
    audio.wake();
    std::thread::sleep(std::time::Duration::from_millis(250));
    anyhow::ensure!(
        audio.controls.rendered_frames.load(Ordering::Relaxed) > paused,
        "Owner did not resume"
    );
    let controls = audio.controls.clone();
    drop(audio);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let stopped = controls.rendered_frames.load(Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    anyhow::ensure!(
        controls.rendered_frames.load(Ordering::Relaxed) == stopped,
        "Owner did not release the output graph"
    );
    anyhow::ensure!(
        controls.render_errors.load(Ordering::Relaxed) == 0,
        "Owner buffer contract failed"
    );
    println!(
        "Background audio owner started, suspended, previewed a quiet effect, resumed and released its graph."
    );
    Ok(())
}

/// These WAVs are inspection artifacts only. The app ships no recordings.
#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
pub(crate) fn write_previews(directory: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(directory)?;
    for level in 0..4 {
        let path = directory.join(if level == 3 {
            "rooftops-scorecard.wav".into()
        } else {
            format!("rooftops-music-{}.wav", level + 1)
        });
        let mut synth = synth::Synth::new(0x676174746f);
        synth.scene(level, level == 3);
        write_wav(&path, 48_000 * 48, |index| {
            let fade = ((48_000 * 48 - index) as f32 / 48_000.).min(1.);
            synth.next(true, 75, 55).map(|sample| sample * fade)
        })?;
    }
    write_effect_previews(directory)
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
pub(crate) fn write_effect_previews(directory: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(directory)?;
    let mut death = death::Death::new();
    death.trigger();
    write_wav(&directory.join("rooftops-death.wav"), 48_000 * 4, |_| {
        [death.next(); 2]
    })?;
    for (name, cue) in [("coin", Cue::Coin), ("shutters", Cue::Shutters)] {
        let mut synth = synth::Synth::new(0x676174746f);
        for _ in 0..4000 {
            synth.next(true, 0, 100);
        }
        synth.cues(cue.bit());
        write_wav(
            &directory.join(format!("rooftops-{name}.wav")),
            48_000,
            |_| synth.next(true, 0, 100),
        )?;
    }
    Ok(())
}

#[cfg(feature = "walkthrough-screenshots")]
fn write_wav(
    path: &std::path::Path,
    frames: u32,
    mut next: impl FnMut(u32) -> [f32; 2],
) -> anyhow::Result<()> {
    use std::io::{BufWriter, Write};
    let mut file = BufWriter::new(std::fs::File::create(path)?);
    let data_bytes = frames * 4;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    for value in [1_u16, 2] {
        file.write_all(&value.to_le_bytes())?;
    }
    file.write_all(&48_000_u32.to_le_bytes())?;
    file.write_all(&192_000_u32.to_le_bytes())?;
    for value in [4_u16, 16] {
        file.write_all(&value.to_le_bytes())?;
    }
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;
    for index in 0..frames {
        for sample in next(index) {
            file.write_all(&((sample.clamp(-1., 1.) * 32767.) as i16).to_le_bytes())?;
        }
    }
    file.flush()?;
    Ok(())
}

#[cfg(feature = "walkthrough-screenshots")]
#[allow(dead_code)]
pub(crate) fn audit(directory: &std::path::Path) -> anyhow::Result<()> {
    synth::audit(directory)
}
