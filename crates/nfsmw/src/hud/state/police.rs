//! What the HUD knows of the police: the phase of the chase, and what the player can know of the cops. The world fills
//! it from the pursuit's authoritative state (`scenes/world/ai/police/intel.rs`), never from the heat alone.
//!
//! Fair knowledge: nothing positional is given without a sighting. A cop's place is given only while that cop has the
//! player in sight at its own sight test (range, facing and a clear line, `ai/collide.rs`), read both ways: a cop that
//! sees the player is one the player can see. The helicopter's place is given only while it has the player in sight.
//! Where the police last saw the player is given as the last-known place with its age: that is the police's knowledge
//! the search works from, not a cop's place.

/// The phase of a chase, as the HUD lays itself out for it.
///
/// The sim separates the search from the cool-down: after the cops lose sight of the player, a seven-second grace runs
/// before the chase counts as lost, and the cool-down meter shows only after it
/// (`blackbox_pursuit::cooldown`, `docs/specs/ai-pursuit.md` sections 4.3 and 6). The grace is the search.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PursuitPhase {
    /// No chase, and no end card up.
    #[default]
    None,
    /// The chase runs and a cop (or the helicopter) has the player in sight.
    Active,
    /// The chase runs, no cop sees the player, and the cool-down meter has not come up: the grace after a lost line.
    Search,
    /// The cool-down meter is up: the cops have lost the player.
    Cooldown,
    /// The player got away; held while the end card is up.
    Escaped,
    /// The player was caught; held through the arrest and its ticket, or while the end card is up.
    Busted,
}

/// What the player can know of the police this frame. See the module for the fair-knowledge rules.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoliceIntel {
    /// The cops that have the player in sight now, render space (x, y), metres.
    pub sightings: Vec<[f32; 2]>,
    /// Where the police last saw the player during the chase, and how long ago; `None` without a chase.
    pub last_known: Option<LastKnown>,
    /// The helicopter, while one flies.
    pub helicopter: Option<HelicopterIntel>,
    /// Dispatch has a roadblock standing in the chase (the radio calls it).
    pub roadblock_reported: bool,
    /// The standing roadblocks, render space (x, y); given only once dispatch reports them.
    pub roadblocks: Vec<[f32; 2]>,
    /// The cost to state of the chase so far.
    pub cost_to_state: u32,
    /// The cop cars that have joined the chase.
    pub units_involved: u32,
}

/// Where the police last saw the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastKnown {
    /// Render space (x, y), metres.
    pub position: [f32; 2],
    /// Seconds since a cop last saw the player, never negative.
    pub age_secs: f32,
}

/// The police helicopter, as far as the player can know it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelicopterIntel {
    /// It has the player in sight.
    pub in_sight: bool,
    /// Its place, render space (x, y); `None` while it does not have the player in sight.
    pub position: Option<[f32; 2]>,
}

impl HelicopterIntel {
    /// The helicopter at `position`, given only while it has the player `in_sight`.
    pub fn new(in_sight: bool, position: [f32; 2]) -> Self {
        Self { in_sight, position: in_sight.then_some(position) }
    }
}

/// The places of the cops that have the player in sight, out of every cop's place and whether it sees the player.
pub fn sighted(cops: impl IntoIterator<Item = ([f32; 2], bool)>) -> Vec<[f32; 2]> {
    cops.into_iter().filter(|(_, sees)| *sees).map(|(at, _)| at).collect()
}

/// The roadblocks the player knows of: every standing one once dispatch reports them, none before.
pub fn known_roadblocks(reported: bool, sites: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    match reported {
        true => sites,
        false => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_cops_that_see_the_player_are_placed() {
        let cops = [([1.0, 2.0], true), ([3.0, 4.0], false), ([5.0, 6.0], true)];
        assert_eq!(sighted(cops), vec![[1.0, 2.0], [5.0, 6.0]]);
        assert!(sighted([([9.0, 9.0], false)]).is_empty(), "no sighting, no place");
    }

    #[test]
    fn the_helicopter_is_placed_only_while_it_sees_the_player() {
        assert_eq!(HelicopterIntel::new(true, [7.0, 8.0]).position, Some([7.0, 8.0]));
        let hidden = HelicopterIntel::new(false, [7.0, 8.0]);
        assert!(!hidden.in_sight);
        assert_eq!(hidden.position, None, "no line of sight, no place");
    }

    #[test]
    fn roadblocks_are_known_once_dispatch_reports_them() {
        assert!(known_roadblocks(false, vec![[1.0, 1.0]]).is_empty());
        assert_eq!(known_roadblocks(true, vec![[1.0, 1.0]]), vec![[1.0, 1.0]]);
    }

    #[test]
    fn without_police_data_nothing_is_known() {
        let intel = PoliceIntel::default();
        assert!(intel.sightings.is_empty() && intel.roadblocks.is_empty());
        assert_eq!((intel.last_known, intel.helicopter), (None, None));
        assert_eq!(crate::hud::HudState::default().pursuit_phase, PursuitPhase::None);
    }
}
