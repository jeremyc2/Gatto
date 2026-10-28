//! Timed poses keep ghosts independent of live physics, input and collectibles.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};

// Bump when either the stored schema or the authored level geometry changes.
const FORMAT_VERSION: u32 = 1;
const SAMPLE_INTERVAL: f64 = 1. / 30.;
static SAVE_GENERATIONS: LazyLock<Mutex<HashMap<PathBuf, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct Pose {
    pub x: f32,
    pub y: f32,
    pub facing: f32,
    pub gait: f32,
    pub squash: f32,
    pub home: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct Frame {
    pub seconds: f64,
    pub level: usize,
    pub returns: usize,
    pub pose: Pose,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Replay {
    pub seconds: f64,
    frames: Vec<Frame>,
}

impl Replay {
    fn valid(&self) -> bool {
        let Some(first) = self.frames.first() else {
            return false;
        };
        let Some(last) = self.frames.last() else {
            return false;
        };
        self.seconds.is_finite()
            && self.seconds > 0.
            && first.seconds == 0.
            && first.level == 0
            && last.seconds == self.seconds
            && last.level == crate::rooftops::LEVELS - 1
            && last.pose.home
            && (0..crate::rooftops::LEVELS)
                .all(|level| self.frames.iter().any(|frame| frame.level == level))
            && self.frames.iter().all(|frame| {
                frame.seconds.is_finite()
                    && frame.seconds >= 0.
                    && frame.seconds <= self.seconds
                    && frame.level < crate::rooftops::LEVELS
                    && [
                        frame.pose.x,
                        frame.pose.y,
                        frame.pose.facing,
                        frame.pose.gait,
                        frame.pose.squash,
                    ]
                    .iter()
                    .all(|v| v.is_finite())
                    && frame.pose.facing.abs() == 1.
                    && frame.pose.gait.abs() <= 3.
                    && (0. ..=1.).contains(&frame.pose.squash)
                    && (0. ..=100_000.).contains(&frame.pose.x)
                    && frame.pose.y.abs() <= 10_000.
            })
            && self.frames.windows(2).all(|pair| {
                pair[0].seconds <= pair[1].seconds
                    && pair[0].level <= pair[1].level
                    && pair[0].returns <= pair[1].returns
            })
    }

    /// Query the shared total-run clock; changing levels never rewinds a ghost.
    #[cfg(test)]
    pub fn pose_at(&self, seconds: f64, level: usize) -> Option<Pose> {
        self.frame_at(seconds)
            .filter(|frame| frame.level == level)
            .map(|frame| frame.pose)
    }

    pub fn frame_at(&self, seconds: f64) -> Option<Frame> {
        if !seconds.is_finite() || seconds < 0. {
            return None;
        }
        let index = self
            .frames
            .partition_point(|frame| frame.seconds <= seconds)
            .checked_sub(1)?;
        let before = self.frames[index];
        let Some(after) = self.frames.get(index + 1).copied() else {
            return Some(before);
        };
        // Respawns and level changes are jumps, never lines through the scenery.
        if before.level != after.level
            || before.returns != after.returns
            || before.pose.home != after.pose.home
            || after.seconds <= before.seconds
        {
            return Some(before);
        }
        let t =
            ((seconds - before.seconds) / (after.seconds - before.seconds)).clamp(0., 1.) as f32;
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Some(Frame {
            pose: Pose {
                x: mix(before.pose.x, after.pose.x),
                y: mix(before.pose.y, after.pose.y),
                gait: mix(before.pose.gait, after.pose.gait),
                squash: mix(before.pose.squash, after.pose.squash),
                ..before.pose
            },
            ..before
        })
    }
}

pub(crate) struct Recording {
    frames: Vec<Frame>,
}

impl Recording {
    pub fn new(initial: Frame) -> Self {
        Self {
            frames: vec![initial],
        }
    }

    pub fn capture(&mut self, frame: Frame) {
        if self.frames.last().is_none_or(|last| {
            frame.seconds - last.seconds >= SAMPLE_INTERVAL
                || frame.level != last.level
                || frame.returns != last.returns
                || frame.pose.home != last.pose.home
        }) {
            self.frames.push(frame);
        }
    }

    pub fn finish(&mut self, final_frame: Frame) -> Option<Replay> {
        self.frames.push(final_frame);
        let replay = Replay {
            seconds: final_frame.seconds,
            frames: std::mem::take(&mut self.frames),
        };
        replay.valid().then_some(replay)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct GameData {
    version: u32,
    pub personal_best: Option<f64>,
    pub replay: Option<Replay>,
}

impl Default for GameData {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            personal_best: None,
            replay: None,
        }
    }
}

impl GameData {
    pub fn path() -> Result<PathBuf> {
        Ok(crate::settings::data_directory()?.join("rooftops.cbor"))
    }

    pub fn has_completed(path: &Path) -> bool {
        #[derive(Deserialize)]
        struct Header {
            version: u32,
            personal_best: Option<f64>,
        }
        // Ignore the replay payload when preferences only need the completion flag.
        std::fs::File::open(path)
            .ok()
            .and_then(|file| ciborium::from_reader::<Header, _>(std::io::BufReader::new(file)).ok())
            .is_some_and(|header| {
                header.version == FORMAT_VERSION
                    && header
                        .personal_best
                        .is_some_and(|seconds| seconds.is_finite() && seconds > 0.)
            })
    }

    pub fn save_generation(path: &Path) -> Result<u64> {
        let generations = SAVE_GENERATIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Rooftops save lock unavailable"))?;
        Ok(*generations.get(path).unwrap_or(&0))
    }

    pub fn reset(path: &Path) -> Result<()> {
        let mut generations = SAVE_GENERATIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Rooftops save lock unavailable"))?;
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let generation = generations.entry(path.to_path_buf()).or_default();
        *generation = generation.wrapping_add(1);
        Ok(())
    }

    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = std::fs::File::open(path)?;
        let data: Self = ciborium::from_reader(std::io::BufReader::new(file))?;
        ensure!(
            data.version == FORMAT_VERSION,
            "Unsupported Rooftops replay version"
        );
        ensure!(
            data.personal_best.is_none_or(|s| s.is_finite() && s > 0.),
            "Invalid best time"
        );
        ensure!(
            data.replay.as_ref().is_none_or(Replay::valid),
            "Invalid ghost recording"
        );
        ensure!(
            data.replay.as_ref().is_none_or(|replay| data
                .personal_best
                .is_some_and(|best| best <= replay.seconds)),
            "Inconsistent best time"
        );
        Ok(data)
    }

    #[cfg(test)]
    pub fn save_progress(path: &Path, seconds: f64, replay: Option<&Replay>) -> Result<Self> {
        Self::save_progress_inner(path, seconds, replay, None)?
            .context("Missing saved Rooftops data")
    }

    pub fn save_if_current(
        path: &Path,
        seconds: f64,
        replay: Option<&Replay>,
        generation: u64,
    ) -> Result<()> {
        Self::save_progress_inner(path, seconds, replay, Some(generation)).map(|_| ())
    }

    fn save_progress_inner(
        path: &Path,
        seconds: f64,
        replay: Option<&Replay>,
        expected: Option<u64>,
    ) -> Result<Option<Self>> {
        let generations = SAVE_GENERATIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Rooftops save lock unavailable"))?;
        if expected.is_some_and(|expected| expected != *generations.get(path).unwrap_or(&0)) {
            return Ok(None);
        }
        ensure!(
            seconds.is_finite() && seconds > 0.,
            "Invalid completion time"
        );
        ensure!(
            replay.is_none_or(|r| r.valid() && r.seconds == seconds),
            "Invalid completed replay"
        );
        let mut data = Self::read(path).unwrap_or_default();
        data.personal_best = Some(data.personal_best.map_or(seconds, |best| best.min(seconds)));
        if let Some(replay) = replay
            && data
                .replay
                .as_ref()
                .is_none_or(|best| replay.seconds < best.seconds)
        {
            data.replay = Some(replay.clone());
        }
        let parent = path.parent().context("Missing Rooftops data directory")?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        ciborium::into_writer(&data, file.as_file_mut())?;
        file.as_file().sync_all()?;
        file.persist(path)?;
        Ok(Some(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(seconds: f64, level: usize, x: f32, home: bool) -> Frame {
        Frame {
            seconds,
            level,
            returns: 0,
            pose: Pose {
                x,
                y: 430.,
                facing: 1.,
                gait: 0.,
                squash: 0.,
                home,
            },
        }
    }

    fn replay(seconds: f64) -> Replay {
        let mut frames = vec![frame(0., 0, 105., false)];
        for level in 0..3 {
            let start = level as f64 * seconds / 3.;
            if level > 0 {
                frames.push(frame(start, level, 105., false));
            }
            frames.push(frame(start + seconds / 6., level, 1000., false));
            frames.push(frame(start + seconds / 3., level, 5000., true));
        }
        frames.last_mut().unwrap().seconds = seconds;
        Replay { seconds, frames }
    }

    #[test]
    fn playback_uses_global_time_across_levels_and_holds_the_finish_pose() {
        let replay = replay(30.);
        assert!(replay.valid());
        assert_eq!(replay.pose_at(0., 0).unwrap().x, 105.);
        assert_eq!(replay.pose_at(2.5, 0).unwrap().x, 552.5);
        assert!(replay.pose_at(10., 0).is_none());
        assert_eq!(replay.pose_at(10., 1).unwrap().x, 105.);
        // A player entering level 2 at t=14 sees the ghost already four seconds in.
        assert_eq!(replay.pose_at(14., 1).unwrap().x, 821.);
        assert!(replay.pose_at(14., 2).is_none());
        assert!(replay.pose_at(21., 1).is_none());
        assert!(replay.pose_at(40., 2).unwrap().home);
        assert_eq!(replay.pose_at(40., 2).unwrap().x, 5000.);
    }

    #[test]
    fn reset_clears_completion_and_ghost_and_blocks_pending_saves() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rooftops.cbor");
        assert!(!GameData::has_completed(&path));
        let old_generation = GameData::save_generation(&path).unwrap();
        GameData::save_if_current(&path, 30., Some(&replay(30.)), old_generation).unwrap();
        assert!(GameData::has_completed(&path));
        GameData::reset(&path).unwrap();
        assert!(!path.exists());
        assert!(!GameData::has_completed(&path));
        GameData::save_if_current(&path, 24., Some(&replay(24.)), old_generation).unwrap();
        assert!(
            !path.exists(),
            "a save queued before reset must not restore the record"
        );
        let empty = GameData::read(&path).unwrap();
        assert!(empty.personal_best.is_none() && empty.replay.is_none());
        let new_generation = GameData::save_generation(&path).unwrap();
        GameData::save_if_current(&path, 33., Some(&replay(33.)), new_generation).unwrap();
        assert!(
            GameData::has_completed(&path),
            "a new completed run can unlock reset again"
        );
        assert_eq!(GameData::read(&path).unwrap().replay.unwrap().seconds, 33.);
    }

    #[test]
    fn respawns_do_not_interpolate_through_rooftops() {
        let mut replay = replay(30.);
        let mut after = frame(3., 0, 105., false);
        after.returns = 1;
        replay
            .frames
            .splice(1..1, [frame(2., 0, 700., false), after]);
        assert_eq!(replay.pose_at(2.9, 0).unwrap().x, 700.);
        assert_eq!(replay.pose_at(3., 0).unwrap().x, 105.);
    }

    #[test]
    fn recording_samples_at_30_hz_and_keeps_pause_and_level_boundaries() {
        let mut recording = Recording::new(frame(0., 0, 105., false));
        for i in 1..=120 {
            recording.capture(frame(i as f64 / 120., 0, i as f32, false));
        }
        assert!(recording.frames.len() <= 31);
        let count = recording.frames.len();
        let last = *recording.frames.last().unwrap();
        for _ in 0..100 {
            recording.capture(last);
        }
        assert_eq!(
            recording.frames.len(),
            count,
            "paused time must not add poses"
        );
        recording.capture(frame(1., 1, 105., false));
        assert_eq!(recording.frames.last().unwrap().level, 1);
        assert_eq!(recording.frames.len(), count + 1);
    }

    #[test]
    fn cbor_round_trip_keeps_only_the_fastest_complete_replay_in_one_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rooftops.cbor");
        assert!(GameData::read(&path).unwrap().personal_best.is_none());
        GameData::save_progress(&path, 30., Some(&replay(30.))).unwrap();
        GameData::save_progress(&path, 45., Some(&replay(45.))).unwrap();
        let stored = GameData::read(&path).unwrap();
        assert_eq!(stored.personal_best, Some(30.));
        assert_eq!(stored.replay.unwrap().seconds, 30.);
        let mut tied = replay(30.);
        tied.frames[1].pose.x = 9999.;
        GameData::save_progress(&path, 30., Some(&tied)).unwrap();
        assert_eq!(
            GameData::read(&path)
                .unwrap()
                .replay
                .unwrap()
                .pose_at(5., 0)
                .unwrap()
                .x,
            1000.,
            "ties keep the original opponent"
        );
        GameData::save_progress(&path, 24., Some(&replay(24.))).unwrap();
        let stored = GameData::read(&path).unwrap();
        assert_eq!(stored.personal_best, Some(24.));
        let stored_replay = stored.replay.unwrap();
        assert_eq!(stored_replay.seconds, 24.);
        assert_eq!(stored_replay.pose_at(0., 0).unwrap().x, 105.);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        for invalid in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(GameData::save_progress(&path, invalid, None).is_err());
        }
        assert_eq!(GameData::read(&path).unwrap().personal_best, Some(24.));
    }

    #[test]
    fn incomplete_replays_corrupt_files_and_unknown_versions_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rooftops.cbor");
        let mut incomplete = replay(30.);
        incomplete.frames.last_mut().unwrap().pose.home = false;
        assert!(GameData::save_progress(&path, 30., Some(&incomplete)).is_err());
        assert!(!path.exists());
        std::fs::write(&path, b"not cbor").unwrap();
        assert!(GameData::read(&path).is_err());
        let data = GameData {
            version: 99,
            ..GameData::default()
        };
        let mut bytes = Vec::new();
        ciborium::into_writer(&data, &mut bytes).unwrap();
        std::fs::write(&path, bytes).unwrap();
        assert!(GameData::read(&path).is_err());
    }
}
