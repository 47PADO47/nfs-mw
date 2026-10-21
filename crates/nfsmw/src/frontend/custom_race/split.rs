//! Split screen's part of what the custom race screens remember: whether the flow is the two-player one, which device each
//! player pressed Start on, their cars and their gearboxes. The original keeps the same in `cFrontendDatabase`
//! (`iNumPlayers`, `SetPlayersJoystickPort`, `GetPlayerSettings(n)->Transmission`, the selected car per player).
//! Spec: `docs/specs/split-screen.md` section 2.

use super::mode::Mode;
use super::state::CarPick;
use crate::input::PlayerDevice;
use crate::scenes::world::SplitSetup;
use crate::settings::Transmission;

/// Which race modes the split-screen mode select lists, in its order: no Lap Knockout and no Tollbooth **[decomp]**
/// (`UIQRModeSelect::Setup`).
pub const MODES: [Mode; 4] = [Mode::Circuit, Mode::Sprint, Mode::Drag, Mode::SpeedTrap];

/// Whether a race of this mode starts at once after its track is picked, with no options page: the drag, the sprint (P2P)
/// and the speed trap have no row left in split screen (no Traffic, Opponents, Difficulty or Catch Up, and no race of the
/// install can be reversed) **[decomp]** (`UIQRTrackSelect`, `EXIT_COMPLETE`).
pub fn skips_options(mode: Mode) -> bool {
    matches!(mode, Mode::Drag | Mode::Sprint | Mode::SpeedTrap)
}

/// The two players of a split-screen race while the front end collects them.
#[derive(Clone, Debug, Default)]
pub struct Split {
    /// The flow is the two-player one (`FEDatabase::IsSplitScreenMode`).
    pub on: bool,
    devices: [Option<PlayerDevice>; 2],
    transmissions: [Option<Transmission>; 2],
    /// The car each player picked, remembered between races.
    pub cars: [CarPick; 2],
    /// The car each player accepted, as the world's loader names it (`folder` or `folder:preset`).
    pub accepted: [Option<String>; 2],
}

impl Split {
    pub(super) const fn new() -> Self {
        Self {
            on: false,
            devices: [None, None],
            transmissions: [None, None],
            cars: [CarPick { list: 0, name: None }, CarPick { list: 0, name: None }],
            accepted: [None, None],
        }
    }

    /// Starts the two-player flow from the Quick Race menu: nobody is seated yet.
    pub fn begin(&mut self) {
        self.on = true;
        self.clear_seats();
    }

    /// Leaves the two-player flow (any other Quick Race entry sets one player, as `UIQRMainMenu` does).
    pub fn end(&mut self) {
        self.on = false;
        self.clear_seats();
    }

    /// Forgets the devices and the gearboxes, as the back chain does when it returns to the options.
    pub fn clear_seats(&mut self) {
        self.devices = [None, None];
        self.transmissions = [None, None];
    }

    /// Seats `device` as player `player` (0 or 1). Player 2 may not take the device of player 1 **[decomp]**
    /// (`uiQRPressStart`); returns whether the seat was taken. Seating player 1 again drops player 2's seat, since the
    /// chain restarts from them.
    pub fn seat(&mut self, player: usize, device: PlayerDevice) -> bool {
        if player > 1 || (player == 1 && self.devices[0] == Some(device)) {
            return false;
        }
        self.devices[player] = Some(device);
        if player == 0 {
            self.devices[1] = None;
            self.transmissions[1] = None;
        }
        true
    }

    /// Takes player `player`'s seat away again (Back from their car select) **[decomp]** (`SetPlayersJoystickPort(1, -1)`).
    pub fn unseat(&mut self, player: usize) {
        if let Some(seat) = self.devices.get_mut(player) {
            *seat = None;
        }
    }

    #[cfg(test)]
    pub fn device(&self, player: usize) -> Option<PlayerDevice> {
        self.devices.get(player).copied().flatten()
    }

    pub fn set_transmission(&mut self, player: usize, transmission: Transmission) {
        if let Some(slot) = self.transmissions.get_mut(player) {
            *slot = Some(transmission);
        }
    }

    pub fn transmission(&self, player: usize) -> Option<Transmission> {
        self.transmissions.get(player).copied().flatten()
    }

    /// Both devices, when both players are seated.
    pub fn seated(&self) -> Option<[PlayerDevice; 2]> {
        Some([self.devices[0]?, self.devices[1]?])
    }

    /// What the front end hands the world: the cars both accepted, the devices both pressed Start on, and the gearboxes
    /// (player 2's is the answer just given). `None` while a seat or a car is missing.
    pub fn setup(&self, second: Transmission) -> Option<SplitSetup> {
        let cars = [self.accepted[0].clone()?, self.accepted[1].clone()?];
        let transmissions = [self.transmission(0).unwrap_or_default(), second];
        Some(SplitSetup { cars, devices: self.seated()?, transmissions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_split_modes_have_no_knockout_and_no_tollbooth() {
        assert_eq!(MODES, [Mode::Circuit, Mode::Sprint, Mode::Drag, Mode::SpeedTrap]);
        assert!(!MODES.contains(&Mode::Knockout) && !MODES.contains(&Mode::Tollbooth));
    }

    #[test]
    fn only_the_circuit_shows_the_options() {
        assert!(!skips_options(Mode::Circuit));
        for mode in [Mode::Sprint, Mode::Drag, Mode::SpeedTrap] {
            assert!(skips_options(mode), "{mode:?}");
        }
    }

    #[test]
    fn the_second_player_cannot_take_the_first_players_device() {
        let mut split = Split::new();
        assert!(split.seat(0, PlayerDevice::Keyboard));
        assert!(!split.seat(1, PlayerDevice::Keyboard), "player 1 holds the keyboard");
        assert_eq!(split.device(1), None);
        assert!(split.seat(1, PlayerDevice::KeyboardArrows));
        assert_eq!(split.seated(), Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]));
    }

    #[test]
    fn seating_the_first_player_again_drops_the_second() {
        let mut split = Split::new();
        split.seat(0, PlayerDevice::Keyboard);
        split.seat(1, PlayerDevice::KeyboardArrows);
        split.set_transmission(1, Transmission::Manual);
        split.seat(0, PlayerDevice::KeyboardArrows);
        assert_eq!((split.device(1), split.transmission(1)), (None, None));
        assert_eq!(split.seated(), None);
    }

    #[test]
    fn the_setup_needs_both_seats_and_both_cars() {
        let mut split = Split::new();
        split.seat(0, PlayerDevice::Keyboard);
        split.accepted = [Some("a".into()), Some("b:p".into())];
        assert_eq!(split.setup(Transmission::Manual), None, "player 2 has no seat");
        split.seat(1, PlayerDevice::KeyboardArrows);
        split.set_transmission(0, Transmission::Manual);
        let setup = split.setup(Transmission::Automatic).expect("a complete setup");
        assert_eq!(setup.cars, ["a".to_owned(), "b:p".to_owned()]);
        assert_eq!(setup.devices, [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows]);
        assert_eq!(setup.transmissions, [Transmission::Manual, Transmission::Automatic]);
    }

    #[test]
    fn unseating_and_ending_forget_the_devices() {
        let mut split = Split::new();
        split.begin();
        split.seat(0, PlayerDevice::Keyboard);
        split.unseat(0);
        assert_eq!(split.device(0), None);
        split.seat(0, PlayerDevice::Keyboard);
        split.end();
        assert!(!split.on);
        assert_eq!(split.device(0), None);
    }
}
