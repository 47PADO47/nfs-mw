//! The parameters of a speech request: what the radio line has to say about the situation.
//!
//! A sentence of the speech database is built from the *parameters* of its event: the car's colour and type, the
//! direction the suspect is heading, how hard a collision was, which unit is calling. The game's speech interface
//! gives each event a fixed list of them; this module names them as the original does ([`Param`], in snake case
//! through [`Param::name`]) and holds the values a request sets ([`SpeechParams`]).
//!
//! Every value is a set of bits, `1 << n` for the nth choice of the parameter ([`values`] names them). A request sets
//! only the parameters it knows; a parameter left out lets any variant of the line through, so a caller that does
//! not know the car's colour still gets a vehicle report (in a random colour). Setting a parameter to 0 names no
//! choice: no recording fits, and the event is not said (which is what the original does with an unset field).
//!
//! The speaker (voice 1 to 9) is not a parameter here: it is [`crate::audio::speech::Request::speaker`], and
//! [`lib_params`] puts it first in the event's argument list where the speech database looks for it.
//! Spec: `docs/specs/speech-sentences.md`.

use ea_audio::speech::Params;

mod signatures;
pub mod values;

use signatures::SIGNATURES;

/// A parameter of a speech event, named as in the game's speech interface (see [`Param::name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Param {
    Intensity,
    YesNo,
    NumSuspects,
    PursuitType,
    SpeakerBattalion,
    SpeakerCallSignId,
    TimeSinceLost,
    CarColor,
    CarType,
    Speed,
    Measurement,
    Direction,
    Encounter,
    LocationRegion,
    Location,
    Code,
    SelfStrategyType,
    BackupType,
    SubjectBattalion,
    SubjectCallSignId,
    DispBackupType,
    DispBackupEta,
    RoadblockType,
    RoadblockEngageType,
    ProjectileType,
    NumUnits,
    RollingStrategyType,
    Position,
    SameNew,
    WorldObjectType,
    AddressGroupType,
    SelfOther,
    EvType,
    BailoutType,
    Region,
    OffroadMomentId,
    FirstSubsequent,
    HeatLevel,
    Jurisdiction,
    HeliSelfStrategyType,
    HeliLostVisual,
    HeliBailoutType,
    HeliHazardAlertType,
    CellCallBucket,
    CellCallNumber,
    SpikebeltPosition,
    OnOffScene,
}

impl Param {
    /// Every parameter.
    pub const ALL: [Param; 47] = [
        Self::Intensity,
        Self::YesNo,
        Self::NumSuspects,
        Self::PursuitType,
        Self::SpeakerBattalion,
        Self::SpeakerCallSignId,
        Self::TimeSinceLost,
        Self::CarColor,
        Self::CarType,
        Self::Speed,
        Self::Measurement,
        Self::Direction,
        Self::Encounter,
        Self::LocationRegion,
        Self::Location,
        Self::Code,
        Self::SelfStrategyType,
        Self::BackupType,
        Self::SubjectBattalion,
        Self::SubjectCallSignId,
        Self::DispBackupType,
        Self::DispBackupEta,
        Self::RoadblockType,
        Self::RoadblockEngageType,
        Self::ProjectileType,
        Self::NumUnits,
        Self::RollingStrategyType,
        Self::Position,
        Self::SameNew,
        Self::WorldObjectType,
        Self::AddressGroupType,
        Self::SelfOther,
        Self::EvType,
        Self::BailoutType,
        Self::Region,
        Self::OffroadMomentId,
        Self::FirstSubsequent,
        Self::HeatLevel,
        Self::Jurisdiction,
        Self::HeliSelfStrategyType,
        Self::HeliLostVisual,
        Self::HeliBailoutType,
        Self::HeliHazardAlertType,
        Self::CellCallBucket,
        Self::CellCallNumber,
        Self::SpikebeltPosition,
        Self::OnOffScene,
    ];

    /// The parameter's name in the game's speech interface (`car_color`, `world_object_type`, ...).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Intensity => "intensity",
            Self::YesNo => "yes_no",
            Self::NumSuspects => "num_suspects",
            Self::PursuitType => "pursuit_type",
            Self::SpeakerBattalion => "speaker_battalion",
            Self::SpeakerCallSignId => "speaker_call_sign_id",
            Self::TimeSinceLost => "time_since_lost",
            Self::CarColor => "car_color",
            Self::CarType => "car_type",
            Self::Speed => "speed",
            Self::Measurement => "measurement",
            Self::Direction => "direction",
            Self::Encounter => "encounter",
            Self::LocationRegion => "location_region",
            Self::Location => "location",
            Self::Code => "code",
            Self::SelfStrategyType => "self_strategy_type",
            Self::BackupType => "backup_type",
            Self::SubjectBattalion => "subject_battalion",
            Self::SubjectCallSignId => "subject_call_sign_id",
            Self::DispBackupType => "disp_backup_type",
            Self::DispBackupEta => "disp_backup_eta",
            Self::RoadblockType => "roadblock_type",
            Self::RoadblockEngageType => "roadblock_engage_type",
            Self::ProjectileType => "projectile_type",
            Self::NumUnits => "num_units",
            Self::RollingStrategyType => "rolling_strategy_type",
            Self::Position => "position",
            Self::SameNew => "same_new",
            Self::WorldObjectType => "world_object_type",
            Self::AddressGroupType => "address_group_type",
            Self::SelfOther => "self_other",
            Self::EvType => "ev_type",
            Self::BailoutType => "bailout_type",
            Self::Region => "region",
            Self::OffroadMomentId => "offroad_moment_id",
            Self::FirstSubsequent => "first_subsequent",
            Self::HeatLevel => "heat_level",
            Self::Jurisdiction => "jurisdiction",
            Self::HeliSelfStrategyType => "heli_self_strategy_type",
            Self::HeliLostVisual => "heli_lost_visual",
            Self::HeliBailoutType => "heli_bailout_type",
            Self::HeliHazardAlertType => "heli_hazard_alert_type",
            Self::CellCallBucket => "cell_call_bucket",
            Self::CellCallNumber => "cell_call_number",
            Self::SpikebeltPosition => "spikebelt_position",
            Self::OnOffScene => "on_off_scene",
        }
    }

    /// Every choice of the parameter at once: the bits a sentence's tests span when they accept anything.
    pub const fn choices(self) -> u32 {
        match self {
            Self::Intensity => 0x3,
            Self::YesNo => 0x3,
            Self::NumSuspects => 0x3,
            Self::PursuitType => 0x1f,
            Self::SpeakerBattalion => 0x1ff,
            Self::SpeakerCallSignId => 0x3fffff,
            Self::TimeSinceLost => 0x3,
            Self::CarColor => 0x1fff,
            Self::CarType => 0x7fffffff,
            Self::Speed => 0xfff,
            Self::Measurement => 0x7,
            Self::Direction => 0xf,
            Self::Encounter => 0x3,
            Self::LocationRegion => 0xf,
            Self::Location => 0x7fffffff,
            Self::Code => 0x3,
            Self::SelfStrategyType => 0x1ff,
            Self::BackupType => 0xf,
            Self::SubjectBattalion => 0xff,
            Self::SubjectCallSignId => 0x1ffffff,
            Self::DispBackupType => 0x7f,
            Self::DispBackupEta => 0x3f,
            Self::RoadblockType => 0xf,
            Self::RoadblockEngageType => 0x3,
            Self::ProjectileType => 0x3,
            Self::NumUnits => 0x3,
            Self::RollingStrategyType => 0xff,
            Self::Position => 0xf,
            Self::SameNew => 0x3,
            Self::WorldObjectType => 0x7f,
            Self::AddressGroupType => 0x1f,
            Self::SelfOther => 0x3,
            Self::EvType => 0x7,
            Self::BailoutType => 0x1f,
            Self::Region => 0xf,
            Self::OffroadMomentId => 0xffffff,
            Self::FirstSubsequent => 0x3,
            Self::HeatLevel => 0x1ff,
            Self::Jurisdiction => 0x3,
            Self::HeliSelfStrategyType => 0x1f,
            Self::HeliLostVisual => 0x1f,
            Self::HeliBailoutType => 0xf,
            Self::HeliHazardAlertType => 0x3f,
            Self::CellCallBucket => 0x1ffff,
            Self::CellCallNumber => 0x3ff,
            Self::SpikebeltPosition => 0x7,
            Self::OnOffScene => 0x3,
        }
    }

    /// The parameter called `name` (case does not matter).
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        Some(match name.as_str() {
            "intensity" => Self::Intensity,
            "yes_no" => Self::YesNo,
            "num_suspects" => Self::NumSuspects,
            "pursuit_type" => Self::PursuitType,
            "speaker_battalion" => Self::SpeakerBattalion,
            "speaker_call_sign_id" => Self::SpeakerCallSignId,
            "time_since_lost" => Self::TimeSinceLost,
            "car_color" => Self::CarColor,
            "car_type" => Self::CarType,
            "speed" => Self::Speed,
            "measurement" => Self::Measurement,
            "direction" => Self::Direction,
            "encounter" => Self::Encounter,
            "location_region" => Self::LocationRegion,
            "location" => Self::Location,
            "code" => Self::Code,
            "self_strategy_type" => Self::SelfStrategyType,
            "backup_type" => Self::BackupType,
            "subject_battalion" => Self::SubjectBattalion,
            "subject_call_sign_id" => Self::SubjectCallSignId,
            "disp_backup_type" => Self::DispBackupType,
            "disp_backup_eta" => Self::DispBackupEta,
            "roadblock_type" => Self::RoadblockType,
            "roadblock_engage_type" => Self::RoadblockEngageType,
            "projectile_type" => Self::ProjectileType,
            "num_units" => Self::NumUnits,
            "rolling_strategy_type" => Self::RollingStrategyType,
            "position" => Self::Position,
            "same_new" => Self::SameNew,
            "world_object_type" => Self::WorldObjectType,
            "address_group_type" => Self::AddressGroupType,
            "self_other" => Self::SelfOther,
            "ev_type" => Self::EvType,
            "bailout_type" => Self::BailoutType,
            "region" => Self::Region,
            "offroad_moment_id" => Self::OffroadMomentId,
            "first_subsequent" => Self::FirstSubsequent,
            "heat_level" => Self::HeatLevel,
            "jurisdiction" => Self::Jurisdiction,
            "heli_self_strategy_type" => Self::HeliSelfStrategyType,
            "heli_lost_visual" => Self::HeliLostVisual,
            "heli_bailout_type" => Self::HeliBailoutType,
            "heli_hazard_alert_type" => Self::HeliHazardAlertType,
            "cell_call_bucket" => Self::CellCallBucket,
            "cell_call_number" => Self::CellCallNumber,
            "spikebelt_position" => Self::SpikebeltPosition,
            "on_off_scene" => Self::OnOffScene,
            _ => return None,
        })
    }
}

/// The most parameters an event takes (besides its speaker).
const CAPACITY: usize = 8;

/// The parameters a request sets, by name. Small and `Copy`: a request carries it by value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpeechParams {
    items: [Option<(Param, u32)>; CAPACITY],
}

impl SpeechParams {
    /// No parameter set: every variant of the line is open.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `param` to `value` (replacing an earlier value). Beyond [`CAPACITY`] parameters nothing is stored.
    pub fn set(&mut self, param: Param, value: u32) {
        let slot = self.items.iter_mut().find(|s| s.is_none_or(|(p, _)| p == param));
        let Some(slot) = slot else { return };
        *slot = Some((param, value));
    }

    /// `self` with `param` set to `value`.
    pub fn with(mut self, param: Param, value: u32) -> Self {
        self.set(param, value);
        self
    }

    /// The value of `param`, if set.
    pub fn get(&self, param: Param) -> Option<u32> {
        self.items.iter().flatten().find(|(p, _)| *p == param).map(|&(_, v)| v)
    }

    /// The parameters set, in the order they were set.
    pub fn iter(&self) -> impl Iterator<Item = (Param, u32)> + '_ {
        self.items.iter().flatten().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.items[0].is_none()
    }
}

/// The argument list of the event called `event` (its lower-case name in the `speech` class, like
/// `setup_vehiclereport`): whether the speaker is the first argument, and the parameters after it. A few events are
/// filed under another family in the attribute database than in the speech interface
/// (`anytimeevents_regainvisualinterrupt` is `interrupts_regainvisualinterrupt` there), so a name that is not found
/// is looked up again without its family.
pub fn signature(event: &str) -> Option<(bool, &'static [Param])> {
    let family_less = |name: &'static str| name.split_once('_').map_or(name, |(_, rest)| rest);
    let exact = SIGNATURES.iter().find(|(name, _, _)| name.eq_ignore_ascii_case(event));
    let (_, speaker, fields) = exact.or_else(|| {
        let tail = event.split_once('_').map_or(event, |(_, rest)| rest);
        SIGNATURES.iter().find(|(name, _, _)| family_less(name).eq_ignore_ascii_case(tail))
    })?;
    Some((*speaker, fields))
}

/// The numbered parameters the speech database tests for `event`: the speaker first when the event takes one, then
/// each parameter in the event's argument order. Parameters the request leaves out stay unset (any variant).
pub fn lib_params(event: &str, speaker: u16, params: &SpeechParams) -> Params {
    let mut out = Params::new();
    let Some((has_speaker, fields)) = signature(event) else {
        // An event the interface list does not know: the voice is still the first argument.
        if speaker != 0 {
            out.set(1, u32::from(speaker));
        }
        return out;
    };
    let mut number = 1u8;
    if has_speaker {
        if speaker != 0 {
            out.set(number, u32::from(speaker));
        }
        number += 1;
    }
    for (offset, field) in fields.iter().enumerate() {
        if let Some(value) = params.get(*field) {
            out.set(number + offset as u8, value);
        }
    }
    out
}

/// The `speaker_call_sign_id` / `subject_call_sign_id` value of call sign number `n` (1 to 20).
pub const fn call_sign(n: u32) -> u32 {
    1 << (n - 1)
}

/// The `location` value of road-name number `n` (1 to 31).
pub const fn location(n: u32) -> u32 {
    1 << (n - 1)
}

/// The `heat_level` value of wanted level `level` (2 to 10).
pub const fn heat_level(level: u32) -> u32 {
    1 << (level - 2)
}

/// The `speed` value for a speed in the player's unit: "over the speed limit" below 100, then one step per 20 up to
/// 300 and above.
pub fn speed_for(speed: f32) -> u32 {
    if speed < 100.0 {
        return values::speed::OVER_SPEED_LIMIT;
    }
    let step = (((speed - 100.0) / 20.0) as u32).min(10);
    1 << (step + 1)
}

#[cfg(test)]
mod tests;
