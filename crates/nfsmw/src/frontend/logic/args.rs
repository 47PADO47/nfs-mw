//! What a screen is opened with: [`Args`], the argument the front end passes when it opens a screen and the commands
//! carry (`logic.rs`).

use super::{Category, PauseCtx, QuitTo, Sub};

/// What a screen is opened with (the original passes an argument and reads the game mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    /// Opened from the pause menu (the screens use their in-game look and return to the pause menu).
    pub pause: bool,
    /// The option categories instead of the screen's usual content (the pause menu shows them in the same package).
    pub options: bool,
    pub category: Category,
    /// Opened in the career's safe house (the safe house menu and the screens it opens; `career-safehouse-2.md`).
    pub safehouse: bool,
    /// The rival the bio shows (a bin, 15 down to 1); 0 for the one of the career's current bin.
    pub rival: u8,
    /// Which icon row `MainMenu_Sub.fng` shows when it is not the option categories or the safe house.
    pub sub: Sub,
    /// What the pause menu offers (the pause menu only).
    pub pause_ctx: PauseCtx,
    /// Opened by the custom race: `Car_Select.fng` then picks the car of the race, not a career car (`custom_race/`).
    pub custom: bool,
    /// Opened in a Blacklist sequence (context bit 17, `0x20000`). The first visit's rival bio enters bin 15 when it
    /// opens this way, and its Accept goes on with the sequence (`flow_first_visit.rs`).
    pub sequence: bool,
    /// Opened by the scripted chase's `JumpToCarLot`: the car lot picks the car the first drive to the safe house is made in
    /// (`flow_prologue.rs`).
    pub first_car: bool,
    /// In split screen, the player (0 or 1) whose Press Start and car select this is (`custom_race/press_start.rs`).
    pub player: u8,
    /// Player 2's car select of a co-op career: the car picked is seated for player 2 (`career_screens/car_select.rs`).
    pub coop: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            pause: false,
            options: false,
            category: Category::Audio,
            safehouse: false,
            rival: 0,
            sub: Sub::Options,
            pause_ctx: PauseCtx { race: false, quit: QuitTo::Main },
            custom: false,
            sequence: false,
            first_car: false,
            player: 0,
            coop: false,
        }
    }
}
