//! A split-screen race in the flow: the second player's Press Start after the first player's car, the start behind the loading
//! screen with both players, and the devices the input layer seats while the world has the window (also a co-op guest's,
//! `flow_coop.rs`). Spec: `docs/specs/split-screen.md` section 2.

use super::*;
use crate::frontend::custom_race::state as custom_state;
use crate::frontend::ids::screen::PRESS_START;
use crate::input::PlayerDevice;
use crate::scenes::world::SplitSetup;
use crate::settings::Transmission;

impl Frontend {
    /// Player 1 chose a car and a gearbox: the gearbox is kept and Press Start for player 2 opens in place of the car select.
    pub(in crate::frontend) fn split_first_car(&mut self, transmission: Transmission, host: &mut Host, env: &mut Env) {
        custom_state::with(|state| state.split.set_transmission(0, transmission));
        self.open_menu(PRESS_START, Args { player: 1, ..Args::default() }, host, env);
    }

    /// Player 2 chose: the race starts as a custom race does (Quit goes to the Quick Race menu), with player 1's car on the
    /// grid first and the setup kept for the world (`world_built`) and for the input layer (`split_seats`). Each player's
    /// gearbox is theirs; the transmission setting is left as it is.
    pub(in crate::frontend) fn start_split_race(
        &mut self,
        race: String,
        options: crate::scenes::world::RaceOptions,
        setup: SplitSetup,
        host: &mut Host,
        env: &mut Env,
    ) {
        self.career.end_play();
        self.origin = QuitTo::QuickRace;
        self.race_options = Some(options);
        self.quick_race = Some(race);
        self.quick_car = Some(setup.cars[0].clone());
        self.split = Some(setup);
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }

    /// The devices the input layer seats now: both players' while a split-screen race is loading, running or paused, and the
    /// guest's while a co-op session has the window. Back in the menus the race is over, the setup is forgotten and the seats
    /// are given up (menu input is allowed from any device). The co-op session is forgotten at the menus too, but the safe
    /// house menu stands before the world and keeps it (`in_safe_house`).
    pub(in crate::frontend) fn split_seats(&mut self) -> Option<[PlayerDevice; 2]> {
        let world = in_world(&self.stage);
        if !world && !self.in_safe_house {
            self.end_coop();
        }
        seats(&self.stage, &mut self.split).or_else(|| coop_seats(world, self.coop.as_ref()))
    }
}

fn in_world(stage: &Stage) -> bool {
    matches!(*stage, Stage::Loading | Stage::Playing | Stage::Paused | Stage::Map)
}

fn seats(stage: &Stage, split: &mut Option<SplitSetup>) -> Option<[PlayerDevice; 2]> {
    if !in_world(stage) {
        *split = None;
    }
    split.as_ref().map(|setup| setup.devices)
}

/// The co-op seats while the world has the window: the devices the two players pressed Start on, whatever they are (a pad,
/// the keyboard or the arrow block for either player).
fn coop_seats(in_world: bool, coop: Option<&super::flow_coop::CoopSession>) -> Option<[PlayerDevice; 2]> {
    coop.filter(|_| in_world).map(|session| session.seats)
}

#[cfg(test)]
mod tests {
    use super::super::flow_coop::{coop_setup, guest_device};
    use super::*;

    fn setup() -> SplitSetup {
        SplitSetup {
            cars: ["a".into(), "b".into()],
            devices: [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows],
            transmissions: [Transmission::Automatic; 2],
        }
    }

    #[test]
    fn the_devices_are_seated_while_the_world_has_the_window_and_given_up_in_the_menus() {
        let mut split = Some(setup());
        for stage in [Stage::Loading, Stage::Playing, Stage::Paused, Stage::Map] {
            assert_eq!(
                seats(&stage, &mut split),
                Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]),
                "{stage:?}"
            );
        }
        assert_eq!(seats(&Stage::Menus, &mut split), None, "the race is over");
        assert!(split.is_none(), "the setup is forgotten with the seats");
        assert_eq!(seats(&Stage::Playing, &mut split), None, "a plain free roam seats nobody");
    }

    #[test]
    fn a_race_started_from_the_menus_is_not_split_screen() {
        let mut none = None;
        assert_eq!(seats(&Stage::Loading, &mut none), None);
    }

    fn session(seats: [PlayerDevice; 2]) -> super::super::flow_coop::CoopSession {
        let setup = coop_setup("A".into(), "car".into(), 1, Transmission::Automatic, guest_device(seats));
        super::super::flow_coop::CoopSession { setup, seats }
    }

    #[test]
    fn co_op_seats_the_devices_both_players_pressed_start_on_while_the_world_has_the_window() {
        let pad = |n| PlayerDevice::Pad(bevy_ecs::prelude::Entity::from_raw_u32(n).unwrap());
        let pairs = [
            [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows],
            [pad(3), PlayerDevice::Keyboard],
            [PlayerDevice::Keyboard, pad(3)],
            [pad(3), pad(4)],
        ];
        for seats in pairs {
            for stage in [Stage::Loading, Stage::Playing, Stage::Paused, Stage::Map] {
                assert_eq!(coop_seats(in_world(&stage), Some(&session(seats))), Some(seats), "{stage:?}");
            }
            assert_eq!(coop_seats(false, Some(&session(seats))), None, "the menus seat nobody");
        }
        assert_eq!(coop_seats(true, None), None, "no session, no seats");
    }
}
