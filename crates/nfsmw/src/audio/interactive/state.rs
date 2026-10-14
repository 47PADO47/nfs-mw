//! What the music is told about the game, and the hooks the pursuit and race code calls.
//!
//! The chase reaches [`MusicInput`] every frame through [`MusicInput::follow`], fed by the world's
//! `pursuit_feed`. The `music` console command overrides it. The races have no caller yet; their hooks stay for
//! when they do.

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

/// The chase as the sound reads it each frame, from the world. `None` while no cops chase. The air-support fields are
/// what the helicopter lines need; the world fills them (`scenes/world/ai/police/world.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PursuitFeed {
    /// The heat level of the chase, 1 to 10.
    pub heat: u32,
    /// How tense the chase is, 0 (the cops lost the player) to 1 (they see it).
    pub intensity: f32,
    /// Seconds since the chase began.
    pub secs: f32,
    /// Seconds of unseen time left before the cops give up.
    pub secs_to_evade: f32,
    /// The player's speed, miles per hour.
    pub player_mph: f32,
    /// A police helicopter is in the chase, flying or leaving.
    pub heli_present: bool,
    /// The rain, 0 (dry) to 1 (heavy), from the scene's weather. Set by the scene, not the traffic.
    pub rain_intensity: f32,
    /// A roadblock of this chase stands.
    pub roadblock_active: bool,
    /// The player hit traffic within the traffic's hit memory.
    pub player_hit: bool,
    /// The cop cars wrecked in the chase.
    pub cops_destroyed: u32,
}

/// The pursuit set for a heat level, 1 (lowest) up. **[guess]**: the four sets are spread evenly over heat levels 1
/// to 5, the range of the original's heat; the data does not say how the original chooses.
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

impl MusicInput {
    pub fn state(&self) -> MusicState {
        self.state
    }

    /// Makes the music follow the chase of this frame: a chase on sets its pursuit set and intensity, and no chase
    /// ends the pursuit. Returns true when a chase has just begun, which is when the speech opens it.
    pub fn follow(&mut self, chase: Option<&PursuitFeed>) -> bool {
        let Some(chase) = chase else {
            self.end_pursuit();
            return false;
        };
        let set = pursuit_set_for_heat(chase.heat as f32);
        if self.state.pursuit.is_none() {
            self.start_pursuit(set, chase.intensity);
            return true;
        }
        self.set_pursuit_set(set);
        self.set_pursuit_intensity(chase.intensity);
        false
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

    #[allow(dead_code)] // the race code has no caller yet
    pub fn start_race(&mut self) {
        self.state.racing = true;
    }

    #[allow(dead_code)] // the race code has no caller yet
    pub fn end_race(&mut self) {
        self.state.racing = false;
    }
}
