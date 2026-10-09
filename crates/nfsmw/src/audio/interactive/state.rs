//! What the music is told about the game, and the hooks the pursuit and race code calls.
//!
//! The pursuit and the races arrive with milestone 7; until then nothing writes to [`MusicInput`] but the `music`
//! console command (which overrides it). The methods below are the whole contract: when the AI and the race
//! rules exist they call these from their own systems.

use bevy_ecs::resource::Resource;

/// The pursuit sets of the music graph (spec `docs/specs/music-graph.md` section 8): sections 1 to 4.
pub const PURSUIT_SETS: u8 = 4;

/// A pursuit in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pursuit {
    /// Which of the four pursuit sets plays, 1 to 4. The original picks it with a property of the event that
    /// starts the pursuit music; what decides the property is not known (heat level is the likely input).
    pub set: u8,
    /// How tense the chase is, 0 (the cops lost you, cooling down) to 1 (surrounded). It steers the graph's
    /// control value (0 to 127), which picks the next bar among the louder or calmer ones.
    pub intensity: f32,
}

/// Everything the music director looks at.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MusicState {
    /// `Some` while cops chase the player.
    pub pursuit: Option<Pursuit>,
    /// A race is on. Races play the licensed songs like free roam, so this changes nothing yet; the field is
    /// where a race-specific behaviour (a finish sting, say) would read it.
    pub racing: bool,
}

impl MusicState {
    #[cfg(test)]
    /// The state with a pursuit of `set` at `intensity`; both are clamped to what the music accepts.
    pub fn chased(set: u8, intensity: f32) -> Self {
        Self { pursuit: Some(Pursuit::new(set, intensity)), racing: false }
    }
}

impl Pursuit {
    pub fn new(set: u8, intensity: f32) -> Self {
        let intensity = if intensity.is_nan() { 0.0 } else { intensity.clamp(0.0, 1.0) };
        Self { set: set.clamp(1, PURSUIT_SETS), intensity }
    }
}

/// The pursuit set for a heat level, 1 (lowest) up. **[guess]**: the four sets are spread evenly over heat levels 1
/// to 5, the range of the original's heat; the data does not say how the original chooses.
#[allow(dead_code)] // for the pursuit code of milestone 7
pub fn pursuit_set_for_heat(heat: f32) -> u8 {
    let heat = if heat.is_nan() { 1.0 } else { heat.clamp(1.0, 5.0) };
    let share = (heat - 1.0) / 4.0;
    (1 + (share * f32::from(PURSUIT_SETS)) as u8).min(PURSUIT_SETS)
}

/// The music's view of the game, kept in step by the game code.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct MusicInput {
    state: MusicState,
}

#[allow(dead_code)] // the hooks are called by the pursuit and race code of milestone 7
impl MusicInput {
    pub fn state(&self) -> MusicState {
        self.state
    }

    /// Cops started chasing: the pursuit music of `set` begins, at `intensity` (0 to 1).
    pub fn start_pursuit(&mut self, set: u8, intensity: f32) {
        self.state.pursuit = Some(Pursuit::new(set, intensity));
    }

    /// The chase got calmer or tenser. Does nothing outside a pursuit.
    pub fn set_pursuit_intensity(&mut self, intensity: f32) {
        let Some(pursuit) = self.state.pursuit.as_mut() else { return };
        *pursuit = Pursuit::new(pursuit.set, intensity);
    }

    /// The heat changed and with it the set that fits. Does nothing outside a pursuit.
    pub fn set_pursuit_set(&mut self, set: u8) {
        let Some(pursuit) = self.state.pursuit.as_mut() else { return };
        *pursuit = Pursuit::new(set, pursuit.intensity);
    }

    /// The pursuit is over, escaped or busted: the pursuit music fades out and the songs return after a delay.
    pub fn end_pursuit(&mut self) {
        self.state.pursuit = None;
    }

    pub fn start_race(&mut self) {
        self.state.racing = true;
    }

    pub fn end_race(&mut self) {
        self.state.racing = false;
    }
}
