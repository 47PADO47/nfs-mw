//! The session settings struct (protocol.md section 7.3) and the partial form a host sends to change it.

use super::codec::DecodeError;
use super::common::{Mode, ensure};
use super::macros::{wire_enum, wire_struct};
use super::text::ServerName;
use super::wire::wire_len;
use crate::loadout::CarRestriction;
use crate::version::ModList;

/// Most bytes of an encoded [`Settings`] (`security-gameplay.md` section 1.2).
pub const MAX_SETTINGS_BYTES: usize = 1024;
/// Fewest and most racers in a session.
pub const PLAYERS_RANGE: std::ops::RangeInclusive<u8> = 2..=16;
/// Most spectators in a session.
pub const MAX_SPECTATORS: u8 = 8;
/// Most traffic density, in percent.
pub const MAX_TRAFFIC_DENSITY: u8 = 100;
/// Highest heat level a session can cap the police at.
pub const MAX_COP_LEVEL: u8 = 5;
/// Most laps of a circuit.
pub const MAX_LAPS: u8 = 10;

wire_enum! {
    /// How cars collide with each other.
    pub enum CollisionMode {
        Full = 0,
        /// Cars pass through each other.
        Ghost = 1,
        /// Ghost until the field spreads out after the start.
        GhostStart = 2,
    }
}

wire_enum! {
    /// What damage does.
    pub enum DamageMode {
        /// Damage shows but does not change the handling.
        Visual = 0,
        Off = 1,
    }
}

wire_enum! {
    /// When a player can join a session that is running.
    pub enum JoinInProgress {
        Closed = 0,
        /// Any time in free roam.
        FreeRoam = 1,
        /// Only between races, into the lobby.
        BetweenRaces = 2,
    }
}

wire_struct! {
    /// The session's settings. At most [`MAX_SETTINGS_BYTES`] encoded.
    pub struct Settings {
        pub name: ServerName,
        /// [`PLAYERS_RANGE`].
        pub max_players: u8,
        /// At most [`MAX_SPECTATORS`].
        pub max_spectators: u8,
        pub mode: Mode,
        /// The track or region id.
        pub track: u32,
        /// At most [`MAX_LAPS`]; 0 for modes without laps.
        pub laps: u8,
        /// Percent, at most [`MAX_TRAFFIC_DENSITY`].
        pub traffic_density: u8,
        /// At most [`MAX_COP_LEVEL`]; 0 for no police.
        pub cop_level_cap: u8,
        pub pursuits: bool,
        pub collision: CollisionMode,
        pub catch_up: bool,
        pub nitrous: bool,
        pub damage: DamageMode,
        pub allowed_cars: CarRestriction,
        pub join_in_progress: JoinInProgress,
        pub password_required: bool,
        pub public: bool,
        pub required_mods: ModList,
        /// Refuse clients of another sim build.
        pub sim_build_match: bool,
    }
    check(settings) { settings.check() }
}

impl Settings {
    /// LAN defaults for a session called `name`: 8 racers, free roam, normal traffic, pursuits on.
    pub fn lan(name: ServerName) -> Self {
        Self {
            name,
            max_players: 8,
            max_spectators: 4,
            mode: Mode::FreeRoam,
            track: 0,
            laps: 0,
            traffic_density: 50,
            cop_level_cap: MAX_COP_LEVEL,
            pursuits: true,
            collision: CollisionMode::Full,
            catch_up: false,
            nitrous: true,
            damage: DamageMode::Visual,
            allowed_cars: CarRestriction::Any,
            join_in_progress: JoinInProgress::FreeRoam,
            password_required: false,
            public: false,
            required_mods: ModList::empty(),
            sim_build_match: false,
        }
    }

    /// Every range rule and the size limit; [`DecodeError::BadValue`] or [`DecodeError::TooLarge`] otherwise.
    pub fn check(&self) -> Result<(), DecodeError> {
        ensure(PLAYERS_RANGE.contains(&self.max_players))?;
        ensure(self.max_spectators <= MAX_SPECTATORS)?;
        ensure(self.laps <= MAX_LAPS)?;
        ensure(self.traffic_density <= MAX_TRAFFIC_DENSITY)?;
        ensure(self.cop_level_cap <= MAX_COP_LEVEL)?;
        if wire_len(self) > MAX_SETTINGS_BYTES {
            return Err(DecodeError::TooLarge);
        }
        Ok(())
    }

    /// The settings with `patch` applied. The result still has to pass [`Settings::check`].
    pub fn apply(&self, patch: &SettingsPatch) -> Self {
        let mut out = self.clone();
        if let Some(name) = &patch.name {
            out.name = name.clone();
        }
        out.max_players = patch.max_players.unwrap_or(out.max_players);
        out.max_spectators = patch.max_spectators.unwrap_or(out.max_spectators);
        out.mode = patch.mode.unwrap_or(out.mode);
        out.track = patch.track.unwrap_or(out.track);
        out.laps = patch.laps.unwrap_or(out.laps);
        out.traffic_density = patch.traffic_density.unwrap_or(out.traffic_density);
        out.cop_level_cap = patch.cop_level_cap.unwrap_or(out.cop_level_cap);
        out.pursuits = patch.pursuits.unwrap_or(out.pursuits);
        out.collision = patch.collision.unwrap_or(out.collision);
        out.catch_up = patch.catch_up.unwrap_or(out.catch_up);
        out.nitrous = patch.nitrous.unwrap_or(out.nitrous);
        out.damage = patch.damage.unwrap_or(out.damage);
        if let Some(allowed) = &patch.allowed_cars {
            out.allowed_cars = allowed.clone();
        }
        out.join_in_progress = patch.join_in_progress.unwrap_or(out.join_in_progress);
        out.password_required = patch.password_required.unwrap_or(out.password_required);
        out.public = patch.public.unwrap_or(out.public);
        out.sim_build_match = patch.sim_build_match.unwrap_or(out.sim_build_match);
        out
    }
}

wire_struct! {
    /// A partial settings struct: `None` keeps the current value. The required mods are not host-editable; they follow
    /// the server's own mod list.
    #[derive(Default)]
    pub struct SettingsPatch {
        pub name: Option<ServerName>,
        pub max_players: Option<u8>,
        pub max_spectators: Option<u8>,
        pub mode: Option<Mode>,
        pub track: Option<u32>,
        pub laps: Option<u8>,
        pub traffic_density: Option<u8>,
        pub cop_level_cap: Option<u8>,
        pub pursuits: Option<bool>,
        pub collision: Option<CollisionMode>,
        pub catch_up: Option<bool>,
        pub nitrous: Option<bool>,
        pub damage: Option<DamageMode>,
        pub allowed_cars: Option<CarRestriction>,
        pub join_in_progress: Option<JoinInProgress>,
        pub password_required: Option<bool>,
        pub public: Option<bool>,
        pub sim_build_match: Option<bool>,
    }
    check(patch) {
        let within = |value: Option<u8>, ok: fn(u8) -> bool| value.is_none_or(ok);
        ensure(
            within(patch.max_players, |n| PLAYERS_RANGE.contains(&n))
                && within(patch.max_spectators, |n| n <= MAX_SPECTATORS)
                && within(patch.laps, |n| n <= MAX_LAPS)
                && within(patch.traffic_density, |n| n <= MAX_TRAFFIC_DENSITY)
                && within(patch.cop_level_cap, |n| n <= MAX_COP_LEVEL)
                && wire_len(patch) <= MAX_SETTINGS_BYTES,
        )
    }
}
