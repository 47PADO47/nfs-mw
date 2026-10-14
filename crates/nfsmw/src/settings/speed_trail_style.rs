//! Which high-speed effect the speed trails setting shows: streaks over the view, or the trail behind the car.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpeedTrailStyle {
    /// Lines over the view that move out from the centre, as the speed rises.
    #[default]
    Streaks,
    /// The wind trail behind the car only.
    Car,
}

impl SpeedTrailStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Streaks => "Speed Streaks",
            Self::Car => "Car Only",
        }
    }
}

impl std::str::FromStr for SpeedTrailStyle {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "streaks" => Ok(Self::Streaks),
            "car" => Ok(Self::Car),
            _ => Err(format!("expected streaks or car, got {s:?}")),
        }
    }
}

impl std::fmt::Display for SpeedTrailStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Streaks => "streaks",
            Self::Car => "car",
        })
    }
}
