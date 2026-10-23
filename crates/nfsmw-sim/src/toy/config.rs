//! How a [`super::ToyWorld`] is set up.

/// The largest roster: player cars take entity ids `0..MAX_PLAYERS`.
pub const MAX_PLAYERS: usize = 16;

/// The first traffic entity id. Police ids follow the last traffic id.
pub const FIRST_BOT_ID: u16 = MAX_PLAYERS as u16;

/// The most traffic cars and the most police cars a world will hold, whatever the config asks for.
pub const MAX_TRAFFIC: usize = 400;
pub const MAX_POLICE: usize = 100;

/// An optional circuit: every player drives the checkpoints in order, `laps` times.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceConfig {
    pub laps: u8,
    /// Checkpoint centres on the ground plane, `[x, z]`. Reaching the last one completes a lap.
    pub checkpoints: Vec<[f32; 2]>,
    /// A checkpoint counts when a car's centre is closer than this, in metres.
    pub radius: f32,
}

impl RaceConfig {
    /// A five-checkpoint loop that starts ahead of the spawn grid and runs round the middle of the plane.
    pub fn circuit(laps: u8) -> Self {
        Self {
            laps,
            checkpoints: vec![[0.0, 80.0], [90.0, 60.0], [90.0, -60.0], [-90.0, -60.0], [-90.0, 60.0]],
            radius: 20.0,
        }
    }
}

/// The knobs of a toy world. Everything else follows from the seed.
#[derive(Debug, Clone, PartialEq)]
pub struct ToyConfig {
    /// Bot-driven traffic cars, clamped to [`MAX_TRAFFIC`].
    pub traffic: usize,
    /// Police cars that chase the nearest player, clamped to [`MAX_POLICE`].
    pub police: usize,
    /// The plane is the square `[-bounds, bounds]` on x and z, in metres.
    pub bounds: f32,
    /// Car types `0..car_types` exist; any other id is refused with `UnknownCar`.
    pub car_types: u32,
    /// The circuit, or `None` for free roam.
    pub race: Option<RaceConfig>,
}

impl Default for ToyConfig {
    fn default() -> Self {
        Self { traffic: 8, police: 2, bounds: 500.0, car_types: 64, race: None }
    }
}

impl ToyConfig {
    /// An empty plane: no traffic and no police, so only the players move.
    pub fn bare() -> Self {
        Self { traffic: 0, police: 0, ..Self::default() }
    }

    /// The config with the numbers made safe: counts clamped, bounds finite and at least 50 m, a race with checkpoints.
    pub(super) fn sanitised(mut self) -> Self {
        self.traffic = self.traffic.min(MAX_TRAFFIC);
        self.police = self.police.min(MAX_POLICE);
        if !self.bounds.is_finite() || self.bounds < 50.0 {
            self.bounds = 50.0;
        }
        self.car_types = self.car_types.min(u32::from(u16::MAX) + 1);
        if let Some(race) = &mut self.race {
            race.laps = race.laps.max(1);
            race.checkpoints.truncate(usize::from(u8::MAX));
            if !race.radius.is_finite() || race.radius < 1.0 {
                race.radius = 1.0;
            }
        }
        if self.race.as_ref().is_some_and(|race| race.checkpoints.is_empty()) {
            self.race = None;
        }
        self
    }
}
