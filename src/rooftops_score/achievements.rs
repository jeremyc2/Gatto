use super::Score;

pub(crate) const NIGHT_EXPRESS_SECONDS: f64 = 75.;

#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Achievement {
    CleanSweep,
    Pacifist,
    NightExpress,
    AllNineIntact,
    DoubleFeature,
    BorrowedLift,
    HomeByMoonlight,
    EmptyPaws,
}

impl Achievement {
    pub const ALL: [Self; 8] = [
        Self::HomeByMoonlight,
        Self::CleanSweep,
        Self::Pacifist,
        Self::NightExpress,
        Self::AllNineIntact,
        Self::EmptyPaws,
        Self::DoubleFeature,
        Self::BorrowedLift,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CleanSweep => "Clean Sweep",
            Self::Pacifist => "Pacifist",
            Self::NightExpress => "Night Express",
            Self::AllNineIntact => "All Nine Intact",
            Self::DoubleFeature => "Double Feature",
            Self::BorrowedLift => "Borrowed Lift",
            Self::HomeByMoonlight => "Home by Moonlight",
            Self::EmptyPaws => "Empty Paws",
        }
    }

    pub fn description(self) -> String {
        match self {
            Self::CleanSweep => "Finish a run with every coin collected and every enemy defeated.",
            Self::Pacifist => "Finish a run without defeating any enemies.",
            Self::NightExpress => {
                return format!(
                    "Finish a run in {} or less.",
                    super::time_label(NIGHT_EXPRESS_SECONDS),
                );
            }
            Self::AllNineIntact => "Finish a run without dying.",
            Self::DoubleFeature => "Bounce from an owl or bat onto a rat in one jump.",
            Self::BorrowedLift => "Bounce off an enemy to the next roof without jumping again.",
            Self::HomeByMoonlight => "Finish all three neighborhoods.",
            Self::EmptyPaws => "Finish a run without collecting a single coin.",
        }
        .to_owned()
    }
}

// These positions are part of the on-disk format. Keep the existing indices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct Achievements(u16);

impl Achievements {
    pub fn contains(self, achievement: Achievement) -> bool {
        self.0 & (1 << achievement as u8) != 0
    }

    pub fn insert(&mut self, achievement: Achievement) {
        self.0 |= 1 << achievement as u8;
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn record_finish_time(&mut self, seconds: f64) {
        self.insert(Achievement::HomeByMoonlight);
        // Use the same whole seconds shown by the race clock and scorecard.
        if seconds.floor() <= NIGHT_EXPRESS_SECONDS {
            self.insert(Achievement::NightExpress);
        }
    }

    pub fn record_score(&mut self, score: Score) {
        self.record_finish_time(score.seconds);
        if score.kills == 0 {
            self.insert(Achievement::Pacifist);
        }
        if score.deaths == 0 {
            self.insert(Achievement::AllNineIntact);
        }
        if score.coins == 0 {
            self.insert(Achievement::EmptyPaws);
        }
    }
}
