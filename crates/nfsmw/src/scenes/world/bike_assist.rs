//! The game's assists of the bikes that are not RIDE 6's: the wheelie from the throttle alone (`easy_wheelie`,
//! `docs/specs/ride6-wheelie-throttle.md`), switched by the setting for the bike the player rides.

use std::sync::atomic::{AtomicBool, Ordering};

use blackbox_vehicle::bike::ThrottleWheelie;

/// Off until the host has read the settings (`app/host.rs`): the physics tests and a bike before that see the reference behaviour.
static EASY_WHEELIE: AtomicBool = AtomicBool::new(false);
static LIVE_DASH: AtomicBool = AtomicBool::new(true);

/// Switch the throttle-only wheelie (the `easy_wheelie` setting); read at every step, so a change reaches a bike on the road.
pub(crate) fn set_easy_wheelie(on: bool) {
    EASY_WHEELIE.store(on, Ordering::Relaxed);
}

/// Switch the live screen of the bikes' instrument clusters (the `live_dash` setting, `docs/modding/ride6-bike-dash.md`); read every frame.
pub(crate) fn set_live_dash(on: bool) {
    LIVE_DASH.store(on, Ordering::Relaxed);
}

/// Whether the live cluster screen is on.
pub(super) fn live_dash() -> bool {
    LIVE_DASH.load(Ordering::Relaxed)
}

/// The throttle-only wheelie's settings for a bike: the game's while the setting is on and the player rides it, the reference's
/// otherwise (a computer driver's bike keeps its throttle and its front wheel down).
pub(in crate::scenes::world) fn throttle_wheelie(player: bool) -> ThrottleWheelie {
    choose(player, EASY_WHEELIE.load(Ordering::Relaxed))
}

fn choose(player: bool, setting: bool) -> ThrottleWheelie {
    match player && setting {
        true => ThrottleWheelie::GAME,
        false => ThrottleWheelie::REFERENCE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_and_the_player_switch_the_assist() {
        assert!(choose(true, true).enabled);
        assert!(!choose(false, true).enabled, "a computer driver does not get it");
        assert_eq!(choose(true, false), ThrottleWheelie::REFERENCE);
    }
}
