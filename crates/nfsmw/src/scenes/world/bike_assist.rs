//! The game's assists of the bikes that are not RIDE 6's: the wheelie from the throttle alone (`easy_wheelie`,
//! `docs/specs/ride6-wheelie-throttle.md`), switched by the setting for the bike the player rides.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

use blackbox_vehicle::air_control::AirAxes;
use blackbox_vehicle::bike::ThrottleWheelie;

thread_local! {
    /// Off until the host has read the settings (`app/host.rs`): a bike before that sees the reference behaviour. Per thread, because
    /// the host and the physics step run on the main thread; a test that builds a `Host` (default settings: on) on its own thread then
    /// cannot switch the assist on under the physics tests that run beside it.
    static EASY_WHEELIE: Cell<bool> = const { Cell::new(false) };
    /// The `air_control` setting, on the same terms (off until the host has read the settings).
    static AIR_CONTROL: Cell<bool> = const { Cell::new(false) };
}
static LIVE_DASH: AtomicBool = AtomicBool::new(true);

/// Switch the throttle-only wheelie (the `easy_wheelie` setting); read at every step, so a change reaches a bike on the road.
pub(crate) fn set_easy_wheelie(on: bool) {
    EASY_WHEELIE.set(on);
}

/// Switch the air control (the `air_control` setting, `docs/specs/air-control.md`); read at every step, so a change reaches a vehicle on the road.
pub(crate) fn set_air_control(on: bool) {
    AIR_CONTROL.set(on);
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
    choose(player, EASY_WHEELIE.get())
}

/// The air control of a vehicle this step: the player's stick while the setting is on, `None` (off) for everything else (a computer driver
/// never steers in the air, and with the setting off the vehicle flies as it always did).
pub(in crate::scenes::world) fn air_control(player: bool, axes: AirAxes) -> Option<AirAxes> {
    choose_air(player, AIR_CONTROL.get(), axes)
}

fn choose_air(player: bool, setting: bool, axes: AirAxes) -> Option<AirAxes> {
    (player && setting).then_some(axes)
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
    fn the_air_control_is_the_players_and_the_settings() {
        let axes = AirAxes { pitch: 1.0, ..AirAxes::ZERO };
        assert_eq!(choose_air(true, true, axes), Some(axes));
        assert_eq!(choose_air(true, false, axes), None, "the setting is off");
        assert_eq!(choose_air(false, true, axes), None, "a computer driver never steers in the air");
        set_air_control(true);
        assert_eq!(air_control(true, axes), Some(axes));
        assert_eq!(air_control(false, axes), None);
        set_air_control(false);
        assert_eq!(air_control(true, axes), None);
    }

    #[test]
    fn the_setting_and_the_player_switch_the_assist() {
        assert!(choose(true, true).enabled);
        assert!(!choose(false, true).enabled, "a computer driver does not get it");
        assert_eq!(choose(true, false), ThrottleWheelie::REFERENCE);
    }
}
