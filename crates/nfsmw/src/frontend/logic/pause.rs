//! Context for the in-game pause carousel.
use super::QuitTo;

/// What the pause menu offers: a race is running (Resume Race and Restart instead of Resume Free Roam), where Quit
/// goes, whether a co-op session is on (End Co-op), and whether it may end now (`coop_locked`: player 2 is racing or
/// chased, `docs/specs/coop-career.md` section 3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseCtx {
    /// The original tuning entry: absent in modes that exclude it, otherwise grey until a path is available.
    pub tuning: Option<bool>,
    /// Absent outside eligible career free roam; false displays the unavailable action.
    pub safe_house: Option<bool>,
    /// A chase is active, including cooldown; the safe-house confirmation warns that its progress is lost.
    pub pursuit: bool,
    pub race: bool,
    pub quit: QuitTo,
    pub coop: bool,
    pub coop_locked: bool,
    /// The career is being played outside a race and has entered the Blacklist: the pause menu lists Blacklist at this stage.
    pub blacklist: Option<u8>,
}
