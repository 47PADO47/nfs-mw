//! Which systems a frame of the world runs: the one switch between single player (the world runs its own rules) and a
//! multiplayer client (the server owns traffic, police, pursuit, races and the career; ADR 0011 section 4). Every rule
//! and AI system of the frame is gated here and nowhere else.

use super::WorldScene;

/// The systems of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSystems {
    /// The shared 60 Hz simulation of single player: every player's physics, the traffic, the police and their sight,
    /// the roadblocks, the pursuit, the race tick (`simulation.rs`).
    pub world_simulation: bool,
    /// The client's frame instead: its own car predicted by the session, everything else from snapshots
    /// (`net_client/`).
    pub prediction: bool,
    /// Races, their triggers and results (`race/`).
    pub races: bool,
    /// The entry zones: safe houses, shops, race starts (`entry.rs`).
    pub entry: bool,
    /// The career's rules (`career/`).
    pub career: bool,
    /// The co-op catch-up prompt (`players/catch_up.rs`).
    pub catch_up: bool,
    /// The second split-screen player (`split/`).
    pub split_screen: bool,
}

impl FrameSystems {
    /// The systems of a single-player (or split-screen, or server) world, and of a multiplayer client's.
    pub const fn for_mode(client: bool) -> Self {
        Self {
            world_simulation: !client,
            prediction: client,
            races: !client,
            entry: !client,
            career: !client,
            catch_up: !client,
            split_screen: !client,
        }
    }
}

impl WorldScene {
    /// The systems this frame runs.
    pub(super) fn frame_systems(&self) -> FrameSystems {
        FrameSystems::for_mode(self.is_net_client())
    }

    /// Whether the world runs as a multiplayer client.
    #[cfg(feature = "multiplayer")]
    pub(super) fn is_net_client(&self) -> bool {
        self.net_client.is_some()
    }

    #[cfg(not(feature = "multiplayer"))]
    pub(super) fn is_net_client(&self) -> bool {
        false
    }

    /// Without multiplayer there is no client frame.
    #[cfg(not(feature = "multiplayer"))]
    pub(super) fn update_net_client(
        &mut self,
        _renderer: &mut dyn blackbox_gfx::RenderBackend,
        _input: &crate::input::ActionState,
        _dt: f32,
    ) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_player_runs_every_rule_and_predicts_nothing() {
        let systems = FrameSystems::for_mode(false);
        assert!(systems.world_simulation && systems.races && systems.entry && systems.career && systems.catch_up);
        assert!(systems.split_screen && !systems.prediction);
    }

    #[test]
    fn a_client_runs_no_rule_of_its_own_and_predicts_its_car() {
        let systems = FrameSystems::for_mode(true);
        assert_eq!(
            systems,
            FrameSystems {
                world_simulation: false,
                prediction: true,
                races: false,
                entry: false,
                career: false,
                catch_up: false,
                split_screen: false,
            }
        );
    }
}
