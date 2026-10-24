//! The rules a multiplayer session sets for the world (`docs/specs/multiplayer/determinism.md`, step 4). The defaults are
//! single player's behaviour, bit for bit; a session switches the rules that would let one machine's local state (its
//! frame time, its camera, its loader thread, the clock) change the shared world.

use super::WorldScene;

/// Which view decides the gameplay rules that ask what a player sees: the crash gate of a hit, the cops' spawn window,
/// the helicopter's camera, which traffic and props are out of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewSource {
    /// The lead's view is the scene's own simulation camera: whatever this machine shows (an original camera view, a
    /// cinematic moment, a NIS, the photo camera or the free camera). Single player.
    #[default]
    LocalCamera,
    /// Every player's view is its car's own chase camera, which the tick steps from the car and the player's look input.
    /// No local presentation state (camera mode, cut scenes, the free camera) reaches the rules. A session.
    PlayerCars,
}

/// What a session changes in the world. Owned by the world (`WorldScene::rules`), read by the code it governs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionRules {
    /// The speedbreaker, the cinematic moments and the `game_speed` option may scale the tick rate. Off in a session:
    /// one player cannot slow time for the others, the tick runs at 1.0 and the speedbreaker is off for every player
    /// (features.md has not decided on a per-car speedbreaker).
    pub time_scale_allowed: bool,
    /// The seed of everything the world draws: the traffic streams (`ai/streams.rs`) and the race opponents' names.
    /// `None` keeps single player's: the traffic's fixed default seed and names drawn from the clock.
    pub seed: Option<u64>,
    /// Which view the camera-dependent rules read.
    pub views: ViewSource,
    /// The traffic's model loads finish in the frame that asks for them, as a warm-up run's do, so the models a spawn can
    /// pick do not depend on the loader thread's timing (the server's model choice).
    pub blocking_models: bool,
    /// Streamed tiles join the world in lockstep: all requested tiles together, in tile order (`residency/lockstep.rs`),
    /// so the ids of their props and the order of their collision do not depend on the disk's speed (the server's
    /// residency).
    pub lockstep_streaming: bool,
    /// A race plays its screens: the intro (the message card, tutorial and intro movies, the intro NIS) before the
    /// countdown, and the race-over message and the post-race screen after the finish. Off in a session: nobody's screen
    /// is the world's to take, so the racers go from the grid to the countdown, and the race ends the moment it is over
    /// (the server's results screen follows).
    pub race_screens: bool,
}

impl Default for SessionRules {
    /// Single player.
    fn default() -> Self {
        Self {
            time_scale_allowed: true,
            seed: None,
            views: ViewSource::LocalCamera,
            blocking_models: false,
            lockstep_streaming: false,
            race_screens: true,
        }
    }
}

impl SessionRules {
    /// The rules of a session started from `seed`: no time scaling, the players' own views, seeded draws, blocking model
    /// loads and lockstep streaming.
    pub fn session(seed: u64) -> Self {
        Self {
            time_scale_allowed: false,
            seed: Some(seed),
            views: ViewSource::PlayerCars,
            blocking_models: true,
            lockstep_streaming: true,
            race_screens: false,
        }
    }

    /// The seed of the race opponents' names: the session's, or one that differs from run to run.
    pub fn name_seed(&self) -> u32 {
        match self.seed {
            Some(seed) => (seed ^ (seed >> 32)) as u32,
            None => clock_seed(),
        }
    }
}

/// A seed that differs from run to run, so the same race does not always bring the same names in single player.
fn clock_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos())
}

impl WorldScene {
    /// The rules the world runs by.
    pub fn session_rules(&self) -> SessionRules {
        self.rules
    }

    /// Sets the rules the world runs by. A session sets them once, before its first tick: a seed restarts the traffic's
    /// random streams from it.
    pub fn set_session_rules(&mut self, rules: SessionRules) {
        self.rules = rules;
        self.residency.set_lockstep(rules.lockstep_streaming);
        let (Some(seed), Some(traffic)) = (rules.seed, self.traffic.as_mut()) else { return };
        traffic.reseed(seed);
    }
}

/// The `session` console command: `on [seed]` sets a session's rules, `off` single player's, no argument shows them
/// and the world's state hash (`state_hash.rs`).
pub(super) fn session_command(scene: &mut WorldScene, args: &[&str]) -> Result<String, String> {
    match args {
        [] => Ok(format!("{:?}; state hash {:016x}", scene.session_rules(), scene.state_hash())),
        ["off"] => {
            scene.set_session_rules(SessionRules::default());
            Ok("single-player rules".into())
        }
        ["on"] => session_on(scene, "0"),
        ["on", seed] => session_on(scene, seed),
        _ => Err("usage: session [on [seed] | off]".into()),
    }
}

fn session_on(scene: &mut WorldScene, seed: &str) -> Result<String, String> {
    let seed = seed.parse::<u64>().map_err(|_| format!("not a seed: {seed}"))?;
    scene.set_session_rules(SessionRules::session(seed));
    Ok(format!("session rules, seed {seed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_single_player() {
        let rules = SessionRules::default();
        assert!(rules.time_scale_allowed && !rules.blocking_models && !rules.lockstep_streaming);
        assert_eq!((rules.seed, rules.views), (None, ViewSource::LocalCamera));
    }

    #[test]
    fn a_session_seed_gives_the_same_names_every_run() {
        let session = SessionRules::session(0x1234_5678_9ABC_DEF0);
        assert_eq!(session.name_seed(), session.name_seed());
        assert_eq!(session.name_seed(), 0x1234_5678 ^ 0x9ABC_DEF0);
        assert!(!session.time_scale_allowed && session.blocking_models && session.lockstep_streaming);
        assert_eq!(session.views, ViewSource::PlayerCars);
    }
}
