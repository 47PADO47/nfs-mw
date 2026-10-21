//! Two players on one grid: what the front end hands the world to start a split-screen race, and the world's side of it.
//! Spec: `docs/specs/split-screen.md`.

use crate::input::PlayerDevice;
use crate::settings::Transmission;

/// The two players of a split-screen race.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitSetup {
    /// The car folder each player chose, player 1 first.
    pub cars: [String; 2],
    /// The device each player holds (`input::SplitInput`).
    pub devices: [PlayerDevice; 2],
    /// Each player's gearbox.
    pub transmissions: [Transmission; 2],
}
