//! The car select in split screen: the header says which player picks, Accept goes on to the other player's Press Start or
//! starts the race, and Back returns to this player's Press Start. Spec: `docs/specs/split-screen.md` section 2.

use super::CarSelect;
use crate::frontend::custom_race::mode::Mode;
use crate::frontend::custom_race::state;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx};
use crate::settings::Transmission;

/// The header group of the package, shown with `Player 1` or `Player 2`; the labels, and the message that plays the "2 players"
/// intro in place of the quick race one **[decomp]** (`UIQRCarSelect`).
const PLAYER_GROUP: u32 = 0xE3FE_27FE;
const PLAYER_LABELS: [u32; 2] = [0x7B07_0984, 0x7B07_0985];
const TWO_PLAYERS: u32 = 0x2CF6_C390;

impl CarSelect {
    /// Whether this screen is one of the two players' selects.
    pub(super) fn is_split(&self) -> bool {
        state::with(|s| s.split.on)
    }

    /// Opens the select for the player: the package plays its "2 players" intro and names the player in the header.
    pub(super) fn show_player(&self, cx: &mut Cx) {
        cx.rt.post_to_package(cx.package, TWO_PLAYERS);
        cx.label_group(PLAYER_GROUP, PLAYER_LABELS[self.player]);
    }

    /// Back: split screen returns to this player's Press Start (player 2's seat is given up); the custom race to the options.
    pub(super) fn back(&self, cx: &mut Cx) {
        if !self.is_split() {
            return cx.send(Command::Switch(screen::TRACK_OPTIONS.into(), Args::default()));
        }
        if self.player == 1 {
            state::with(|s| s.split.unseat(1));
        }
        let args = Args { player: self.player as u8, ..Args::default() };
        cx.send(Command::Switch(screen::PRESS_START.into(), args));
    }

    /// The car is accepted: the transmission question (not for a drag race, which uses the setting), then on to the other
    /// player's Press Start, or the race after player 2's.
    pub(super) fn start_split(&mut self, cx: &mut Cx) {
        let Some(entry) = self.entry().cloned() else { return };
        let player = self.player;
        let car = match &entry.preset {
            Some(preset) => format!("{}:{preset}", entry.car.folder),
            None => entry.car.folder.clone(),
        };
        state::with(|s| s.split.accepted[player] = Some(car));
        // A select opened by hand, with no track or no seats, has no race to go on to.
        if player == 1 && state::with(|s| s.event().is_none() || s.split.setup(Transmission::Automatic).is_none()) {
            return;
        }
        let go = move |transmission| after_car(player, transmission);
        if state::with(|s| s.mode) == Mode::Drag {
            return cx.send(go(cx.settings.transmission));
        }
        cx.send(Command::Confirm(self.transmission_question(cx, go)));
    }
}

/// What the player's transmission answer does: player 1's goes on to player 2's Press Start, player 2's starts the race
/// with the options and the cars of the state.
fn after_car(player: usize, transmission: Transmission) -> Command {
    if player == 0 {
        return Command::SplitFirstCar(transmission);
    }
    let started = state::with(|s| {
        let setup = s.split.setup(transmission)?;
        Some(Command::StartSplitRace {
            race: s.event()?.to_owned(),
            options: s.settings().split_options(s.mode),
            setup,
        })
    });
    started.unwrap_or(Command::SaveSettings)
}
