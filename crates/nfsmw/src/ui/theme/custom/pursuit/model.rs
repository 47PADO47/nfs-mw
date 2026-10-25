//! What the pursuit panel shows on one frame (`docs/specs/custom-hud/pursuit.md` section 2), as plain data. Each field
//! is present only where the chase computes it, the detail level asks for it and `[custom_hud] pursuit_info` allows it;
//! with `pursuit_info` off only the status and the cooldown bar remain, the minimum needed to play.

use super::super::memory::heat_level;
use super::super::mode::Detail;
use crate::hud::{HudState, PursuitPhase};

/// The status the panel names, for the three phases of a running chase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// A cop has the player in sight.
    Active,
    /// Contact is lost; the cooldown meter is not up yet.
    Search,
    /// The cooldown meter is up and filling.
    Cooldown,
}

impl Status {
    /// The status of `phase`; `None` when no chase runs (the panel leaves at the end of a chase too).
    pub fn of(phase: PursuitPhase) -> Option<Self> {
        match phase {
            PursuitPhase::Active => Some(Status::Active),
            PursuitPhase::Search => Some(Status::Search),
            PursuitPhase::Cooldown => Some(Status::Cooldown),
            PursuitPhase::None | PursuitPhase::Escaped | PursuitPhase::Busted => None,
        }
    }
}

/// The panel's fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelModel {
    pub status: Status,
    /// The heat level, the game's own number.
    pub heat: Option<u32>,
    /// Seconds since the chase began; Balanced and Advanced.
    pub duration: Option<f32>,
    /// The bounty the chase has earned so far (career only); Balanced and Advanced.
    pub bounty: Option<u32>,
    /// The cooldown meter, 0 to 1, only while the phase is `Cooldown`.
    pub cooldown: Option<f32>,
    /// Where the police last saw the player, seconds ago; only in `Search`, where it replaces the tracking.
    pub last_seen: Option<f32>,
    /// How far the hiding spot has latched, 0 to 1, while one hides the player.
    pub hiding: Option<f32>,
    /// The bust meter, 0 to 1, while above zero.
    pub bust: Option<f32>,
    /// The chase's cost to the state; Advanced.
    pub cost_to_state: Option<u32>,
    /// The cop cars that joined the chase and those wrecked; Advanced.
    pub units: Option<(u32, u32)>,
}

impl PanelModel {
    /// The panel of `hud` at `detail`; `None` when no chase runs.
    pub fn of(hud: &HudState, detail: Detail, pursuit_info: bool) -> Option<Self> {
        let status = Status::of(hud.pursuit_phase)?;
        let chase = hud.pursuit.as_ref();
        let cooldown = chase.map(|chase| chase.evade.clamp(0.0, 1.0)).filter(|_| status == Status::Cooldown);
        let bare = Self {
            status,
            heat: None,
            duration: None,
            bounty: None,
            cooldown,
            last_seen: None,
            hiding: None,
            bust: None,
            cost_to_state: None,
            units: None,
        };
        if !pursuit_info {
            return Some(bare);
        }
        let more = detail >= Detail::Balanced;
        let all = detail == Detail::Advanced;
        let last_seen = hud.police_intel.last_known.map(|known| known.age_secs).filter(|_| status == Status::Search);
        Some(Self {
            heat: heat_level(hud),
            duration: chase.map(|chase| chase.secs).filter(|_| more),
            bounty: hud.context.chase_bounty.filter(|_| more),
            last_seen,
            hiding: chase.and_then(|chase| chase.hiding),
            bust: chase.map(|chase| chase.busted.clamp(0.0, 1.0)).filter(|bust| *bust > 0.0),
            cost_to_state: all.then_some(hud.police_intel.cost_to_state),
            units: chase.map(|chase| (hud.cops_engaged, chase.cops_destroyed)).filter(|_| all),
            ..bare
        })
    }

    /// The heat counts as high: the panel steps up its contrast and the number's weight, never flashing.
    pub fn high_heat(&self) -> bool {
        self.heat.is_some_and(|heat| heat >= HIGH_HEAT)
    }
}

/// The heat level from which the panel shows escalation.
pub const HIGH_HEAT: u32 = 4;
/// The heat bar's segments; a level above fills the bar and shows its number.
pub const HEAT_SEGMENTS: usize = 5;
