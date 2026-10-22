//! The effects of the final race's events: one method per effect. The world plays them (`final_race.rs`), and the tests
//! record them. The rules are `nfsmw_career::story::FinalRace`; their events come in the order they must be played.

use nfsmw_career::story::{FinalRaceEvent, GpsTarget};

/// What the final race does to the world, one method per effect.
pub(super) trait FinalEffects {
    /// Places phone call `id` (`ECellCall`, through the phone).
    fn place_call(&mut self, id: u16);
    /// Opens `SCENERY_GROUP_DOOR`.
    fn open_door(&mut self);
    /// Sets the GPS, and the trigger icon, to `barrier_area`.
    fn gps_to_barrier(&mut self);
    /// Clears the GPS once the player is in `barrier_area`.
    fn hide_gps(&mut self);
    /// Plays the outro scene `scene` at the win marker, not skippable.
    fn play_outro(&mut self, scene: &str);
    /// Lets the pursuit go (`EBailPursuit`).
    fn bail_pursuit(&mut self);
    /// Totals the player's racer: the race is lost.
    fn total_racer(&mut self);
}

/// How the race stands after a batch of events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Settled {
    /// Nothing is settled.
    Running,
    /// The race is won. Its outro plays, and the race ends when the outro is over.
    Won,
    /// The racer was totalled: the race is lost.
    Lost,
}

/// Plays `events` in order and says how the race stands. A totalled racer stops the batch: nothing after it plays.
pub(super) fn apply_events(fx: &mut dyn FinalEffects, events: &[FinalRaceEvent]) -> Settled {
    let mut settled = Settled::Running;
    for &event in events {
        match event {
            FinalRaceEvent::PlaceCall(id) => fx.place_call(id),
            FinalRaceEvent::OpenDoor => fx.open_door(),
            FinalRaceEvent::GpsTo(GpsTarget::BarrierArea) => fx.gps_to_barrier(),
            FinalRaceEvent::HideGps => fx.hide_gps(),
            FinalRaceEvent::Won => settled = Settled::Won,
            FinalRaceEvent::PlayOutroNis(scene) => fx.play_outro(scene),
            FinalRaceEvent::BailPursuit => fx.bail_pursuit(),
            FinalRaceEvent::Wrecked => {
                fx.total_racer();
                return Settled::Lost;
            }
        }
    }
    settled
}
