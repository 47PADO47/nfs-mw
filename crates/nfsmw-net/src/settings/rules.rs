//! The session rules on a settings struct and on a host's patch of it.

use super::Field;
use super::pairs::value;
use crate::loadout::CarRestriction;
use crate::message::{
    MAX_COP_LEVEL, MAX_LAPS, MAX_SETTINGS_BYTES, MAX_SPECTATORS, MAX_TRAFFIC_DENSITY, Mode, PLAYERS_RANGE,
    SessionPhase, Settings, SettingsPatch, wire_len,
};
use crate::roles::{Command, Rank, permits};

/// Why settings or a change of them were refused. A refused change leaves the settings as they were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsError {
    /// The field's value is out of range or breaks a rule with another field.
    Invalid(Field),
    /// The encoded struct is over [`MAX_SETTINGS_BYTES`].
    TooLarge,
    /// The sender's rank may not change settings (a spoofed host command; count it as a violation).
    NotPermitted,
    /// The field cannot change between loading and the end of the race.
    Locked(Field),
    /// The new limit is below the number of players or spectators already in the session.
    BelowCount(Field),
    /// `password_required` was turned on while the server holds no password.
    NoPassword,
    /// The flat form named a key that is not a [`Field`].
    UnknownKey,
}

/// What a patch is checked against besides the current settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchContext {
    /// The sender's rank.
    pub rank: Rank,
    /// The session phase now.
    pub phase: SessionPhase,
    /// Racers and cops in the session (held reconnect slots included).
    pub racers: usize,
    /// Spectators in the session.
    pub spectators: usize,
    /// Whether the server holds a room password.
    pub has_password: bool,
}

/// Checks every range of protocol-messages.md 7.3 and the session rules: a lap mode needs at least one lap, pursuits
/// need a police cap above zero, an allowed-car list is not empty, drag races and human cops are not offered (ADR
/// 0011 section 5), and a custom mode needs a required mod that provides it.
pub fn validate(settings: &Settings) -> Result<(), SettingsError> {
    let ranges = [
        (PLAYERS_RANGE.contains(&settings.max_players), Field::MaxPlayers),
        (settings.max_spectators <= MAX_SPECTATORS, Field::MaxSpectators),
        (settings.laps <= MAX_LAPS, Field::Laps),
        (settings.traffic_density <= MAX_TRAFFIC_DENSITY, Field::TrafficDensity),
        (settings.cop_level_cap <= MAX_COP_LEVEL, Field::CopLevelCap),
        (mode_offered(settings), Field::Mode),
        (!has_laps(settings.mode) || settings.laps > 0, Field::Laps),
        (!settings.pursuits || settings.cop_level_cap > 0, Field::CopLevelCap),
        (!matches!(&settings.allowed_cars, CarRestriction::Cars(cars) if cars.is_empty()), Field::AllowedCars),
    ];
    if let Some((_, field)) = ranges.iter().find(|(ok, _)| !ok) {
        return Err(SettingsError::Invalid(*field));
    }
    if wire_len(settings) > MAX_SETTINGS_BYTES {
        return Err(SettingsError::TooLarge);
    }
    Ok(())
}

/// Whether `mode` counts laps (circuit and knockout).
pub fn has_laps(mode: Mode) -> bool {
    matches!(mode, Mode::Circuit | Mode::Knockout)
}

/// Applies `patch` to `current` for a sender described by `context`. Returns the new settings and the fields whose
/// value changed, in [`Field::ALL`] order.
///
/// Order: the rank (only host and admins), the phase lock (from `Loading` to `Running` only the name, the spectator
/// limit, the join policy, the password flag and the public flag may change), the head counts, the password, then
/// [`validate`] on the result.
pub fn apply_patch(
    current: &Settings,
    patch: &SettingsPatch,
    context: &PatchContext,
) -> Result<(Settings, Vec<Field>), SettingsError> {
    if !permits(context.rank, Command::ChangeSettings) {
        return Err(SettingsError::NotPermitted);
    }
    let touched = patched(patch);
    if !editable_phase(context.phase)
        && let Some(field) = touched.iter().find(|field| !field.live_editable())
    {
        return Err(SettingsError::Locked(*field));
    }
    let next = current.apply(patch);
    if usize::from(next.max_players) < context.racers {
        return Err(SettingsError::BelowCount(Field::MaxPlayers));
    }
    if usize::from(next.max_spectators) < context.spectators {
        return Err(SettingsError::BelowCount(Field::MaxSpectators));
    }
    if next.password_required && !context.has_password {
        return Err(SettingsError::NoPassword);
    }
    validate(&next)?;
    let changed = Field::ALL.into_iter().filter(|field| value(*field, current) != value(*field, &next)).collect();
    Ok((next, changed))
}

/// Whether every field may change in `phase`: the lobby and the results screen.
fn editable_phase(phase: SessionPhase) -> bool {
    matches!(phase, SessionPhase::Lobby | SessionPhase::Results)
}

/// The fields a patch sets, in [`Field::ALL`] order.
fn patched(patch: &SettingsPatch) -> Vec<Field> {
    let set = [
        (patch.name.is_some(), Field::Name),
        (patch.max_players.is_some(), Field::MaxPlayers),
        (patch.max_spectators.is_some(), Field::MaxSpectators),
        (patch.mode.is_some(), Field::Mode),
        (patch.track.is_some(), Field::Track),
        (patch.laps.is_some(), Field::Laps),
        (patch.traffic_density.is_some(), Field::TrafficDensity),
        (patch.cop_level_cap.is_some(), Field::CopLevelCap),
        (patch.pursuits.is_some(), Field::Pursuits),
        (patch.collision.is_some(), Field::Collision),
        (patch.catch_up.is_some(), Field::CatchUp),
        (patch.nitrous.is_some(), Field::Nitrous),
        (patch.damage.is_some(), Field::Damage),
        (patch.allowed_cars.is_some(), Field::AllowedCars),
        (patch.join_in_progress.is_some(), Field::JoinInProgress),
        (patch.password_required.is_some(), Field::PasswordRequired),
        (patch.public.is_some(), Field::Public),
        (patch.sim_build_match.is_some(), Field::SimBuildMatch),
    ];
    set.into_iter().filter(|(on, _)| *on).map(|(_, field)| field).collect()
}

/// Whether the session offers `settings.mode`: not drag (MP-04) or cops against racers (MP-08), and a custom mode only
/// with a sim-affecting required mod.
fn mode_offered(settings: &Settings) -> bool {
    match settings.mode {
        Mode::Drag | Mode::CopsVsRacers => false,
        Mode::Custom(_) => settings.required_mods.iter().any(|entry| entry.affects_sim()),
        _ => true,
    }
}
