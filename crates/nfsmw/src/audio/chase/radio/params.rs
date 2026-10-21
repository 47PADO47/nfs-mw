//! The machine's typed parameters turned into the speech interface's: each value becomes the bit its parameter takes,
//! and a parameter the event's argument list does not have is left out. Spec: `docs/specs/speech-sentences.md` section 6.

use blackbox_pursuit::speech as m;
use m::Line;

use super::tables::{address_group_of_battalion, address_group_of_region, place};
use crate::audio::speech::params::{self, Param, SpeechParams, signature, values};

/// The event of each line, by the lower-case name the attribute database gives it (family, an underscore, collection).
pub fn event_name(line: Line) -> &'static str {
    match line {
        Line::Spotter => "setup_spotter",
        Line::SpotterWanted => "setup_spotterwanted",
        Line::SpotterReply => "setup_spotterreply",
        Line::AttemptVehicleStop => "setup_attmptvehstp",
        Line::GoAhead => "setup_dispgoahead",
        Line::PrimaryEngage => "setup_primaryengage",
        Line::InitiatePursuit => "setup_initpursuit",
        Line::SuspectConfirmed => "setup_suspectconfirmed",
        Line::ReinitiatePursuit => "setup_reinitpursuit",
        Line::VehicleReport => "setup_vehiclereport",
        Line::LocationReport => "setup_locationreport",
        Line::SelfStrategy => "setup_selfstrategy",
        Line::InitialCallForBackup => "setup_initialcallforbu",
        Line::InitialCallForBackupMulti => "setup_initialcallforbu_ms",
        Line::Bullhorn => "setup_bullhorn",
        Line::VehicleDescription => "setup_dispvehdescrip",
        Line::NoVehicleDescription => "setup_dispnovehdescrip",
        Line::MoreDetails => "setup_moredetails",
        Line::DriverHistory => "anytimeevents_driverhistory",
        Line::CallForBackup => "backup_callforbu",
        Line::UnitBackupReply => "backup_unitbureply",
        Line::DispBackupReply => "backup_dispbackupreply",
        Line::CallForSwarming => "backup_callforswarming",
        Line::DispBackupEta => "backup_dispbueta",
        Line::BackupReminder => "backup_bureminder",
        Line::NegativeBackupReply => "backup_negativebureply",
        Line::DispBackupUpdate => "backup_dispbackupupdate",
        Line::BackupArrives => "backup_buarrives",
        Line::CallForRoadblock => "staticroadblock_callforrb",
        Line::RoadblockReminder => "staticroadblock_rbreminder",
        Line::NegativeRoadblockReply => "staticroadblock_negativerbreply",
        Line::DispRoadblockReply => "staticroadblock_disprbreply",
        Line::DispRoadblockUpdate => "staticroadblock_disprbupdate",
        Line::PursuitApproaching => "staticroadblock_pursuitapproaching",
        Line::RoadblockApproach => "staticroadblock_rbapproach",
        Line::RoadblockEngage => "staticroadblock_rbengage",
        Line::RoadblockAverted => "staticroadblock_rbaverted",
        Line::CallForSubRoadblock => "staticroadblock_callforrb_sub",
        Line::DispSubRoadblock => "staticroadblock_dispsubrb",
        Line::RoadblockWarning => "extracops_rbwarning",
        Line::RoadblockPosition => "extracops_rbposition",
        Line::ExtraRoadblockEngage => "extracops_extrarbengage",
        Line::ExtraRoadblockAverted => "extracops_extrarbaverted",
        Line::InitiateStrategy => "rollingstrategy_initstrategy",
        Line::CallToPosition => "rollingstrategy_calltoposition",
        Line::CallToPositionReminder => "rollingstrategy_calltopositionrem",
        Line::StrategyExecute => "rollingstrategy_strategyexecute",
        Line::AnticipateFail => "outcome_anticipatefail",
        Line::AnticipateSuccess => "outcome_anticipatesuccess",
        Line::OutcomeFail => "outcome_outcomefail",
        Line::StrategyReset => "outcome_strategyreset",
        Line::BullhornArrest => "arrest_bullhornarrest",
        Line::Arrest => "arrest_arrest",
        Line::DispArrestReply => "arrest_disparrestreply",
        Line::CollisionWorld => "anytimeevents_collisionworld",
        Line::CollWorldCivi => "anytimeevents_collworld_civi",
        Line::CollWorldSpin => "anytimeevents_collworld_spin",
        Line::CollWorldAir => "anytimeevents_collworld_air",
        Line::CollWorldFlip => "anytimeevents_collworld_flip",
        Line::SuspectUTurn => "anytimeevents_suspectuturn",
        Line::SuspectOutrun => "anytimeevents_suspectoutrun",
        Line::SuspectBrake => "anytimeevents_suspectbrake",
        Line::SuspectBehaviour => "anytimeevents_suspectbehaviour",
        Line::OffroadMoment => "anytimeevents_offroadmoment",
        Line::DirectionHigh => "anytimeevents_directionhigh",
        Line::LostVisual => "anytimeevents_lostvisual",
        Line::RegainVisual => "anytimeevents_regainvisual",
        Line::LostSuspect => "anytimeevents_lostsuspect",
        Line::DispBreakAway => "anytimeevents_dispbreakaway",
        Line::DispTimeExpired => "anytimeevents_disptimeexpired",
        Line::DispPursuitEscalation => "anytimeevents_disppursuitescalation",
        Line::DispPursuitEscalationGeneric => "anytimeevents_disppursescgen",
        Line::DispPursuitUpdate => "anytimeevents_disppursuitupdate",
        Line::PursuitUpdateReply => "anytimeevents_pursuitupdaterep",
        Line::Spotted => "anytimeevents_spotted",
        Line::FocusChange => "anytimeevents_focuschange",
        Line::HeatJump => "anytimeevents_heatjump",
        Line::DispJurisShift => "anytimeevents_dispjurisshift",
        Line::WeatherReport => "anytimeevents_weatherreport",
        Line::Disp911Report => "anytimeevents_disp911report",
        Line::Unit911Reply => "anytimeevents_unit911reply",
        Line::UnitDisabled => "anytimeevents_unitdisabled",
        Line::CallForEv => "anytimeevents_callforev",
        Line::IntentToRam => "anytimeevents_intenttoram",
        Line::Bailout => "anytimeevents_bailout",
        Line::BailoutDeny => "anytimeevents_bailoutdeny",
        Line::Interrupt => "interrupts_interrupt",
        Line::InterruptExpletive => "interrupts_interruptram",
        Line::InterruptViolent => "interrupts_interruptramhigh",
        Line::RamRearEnd => "interrupts_interruptram_re",
        Line::RamHeadOn => "interrupts_interruptram_ho",
        Line::RamSideSwipe => "interrupts_interruptram_ss",
        Line::RamTBone => "interrupts_interruptram_tb",
        Line::Acknowledge => "acknowledge",
        Line::SwarmingReply => "extracops_swarmingreply",
        Line::SwarmingReplyFollow => "extracops_swarmingreplyfollow",
        Line::QuadrantForming => "extracops_quadrentforming",
        Line::SuspectPossiblyGone => "extracops_suspectpossiblygone",
        Line::QuadrantMoving => "extracops_quadrentmoving",
        Line::OtherLead => "extracops_otherlead",
        Line::PossibleSuspect => "extracops_possiblesuspect",
        Line::WrongSuspect => "extracops_wrongsuspect",
        Line::HeliSelfStrategy => "helispecific_heliselfstrategy",
        Line::HeliLostVisual => "helispecific_helilostvisual",
        Line::HeliIntentToBail => "helispecific_heliintenttobail",
        Line::HeliBailout => "helispecific_helibailout",
        Line::HeliSwarming => "helispecific_heliswarming",
        Line::HeliSpotter => "helispecific_helispotter",
        Line::HeliHazardAlert => "helispecific_helihazardalert",
        Line::HeliQuadrant => "helispecific_heliquadrent",
        Line::HeliQuadrantMoving => "helispecific_heliquadrentmoving",
        Line::HeliBullhornArrest => "helispecific_helibullhornarrest",
    }
}

/// Sets `param` to `value` when the event takes it.
struct Fill<'a> {
    fields: &'a [Param],
    out: SpeechParams,
}

impl Fill<'_> {
    fn set(&mut self, param: Param, value: u32) {
        if self.fields.contains(&param) {
            self.out.set(param, value);
        }
    }

    fn takes(&self, param: Param) -> bool {
        self.fields.contains(&param)
    }
}

/// The speech parameters of `params` for `line`: only those the event's argument list has.
pub fn speech_params(line: Line, p: &m::Params) -> SpeechParams {
    let Some((_, fields)) = signature(event_name(line)) else { return SpeechParams::new() };
    let mut f = Fill { fields, out: SpeechParams::new() };
    describe(&mut f, p);
    places(&mut f, p);
    strategy(&mut f, p);
    incidents(&mut f, p);
    roadblocks(&mut f, p);
    // Every event that takes a ten-code is told not to use one (the original's actors always do).
    f.set(Param::Code, values::code::DONT_USE_10_CODE);
    // The driver history is always the coastal one.
    f.set(Param::Region, values::region::COASTAL);
    f.out
}

fn describe(f: &mut Fill, p: &m::Params) {
    if let Some(v) = p.pursuit_type {
        f.set(Param::PursuitType, pursuit_type(v));
    }
    if let Some(v) = p.suspects {
        let bit = match v {
            m::Suspects::One => values::num_suspects::ONE_SUSPECT,
            m::Suspects::Multiple => values::num_suspects::MULTIPLE_SUSPECTS,
        };
        f.set(Param::NumSuspects, bit);
    }
    if let Some(v) = p.battalion {
        f.set(Param::SpeakerBattalion, u32::from(v.0));
    }
    if let Some(n) = p.call_sign {
        f.set(Param::SpeakerCallSignId, params::call_sign(u32::from(n)));
    }
    if let Some(unit) = p.subject {
        f.set(Param::SubjectBattalion, u32::from(unit.battalion.0));
        f.set(Param::SubjectCallSignId, params::call_sign(u32::from(unit.call_sign)));
    }
    if let Some(v) = p.intensity {
        let bit = match v {
            m::Intensity::Normal => values::intensity::NORMAL,
            m::Intensity::High => values::intensity::HIGH,
        };
        f.set(Param::Intensity, bit);
    }
    if let Some(yes) = p.yes {
        f.set(Param::YesNo, if yes { values::yes_no::YES_TRUE } else { values::yes_no::NO_FALSE });
    }
    if let Some(v) = p.lost_for {
        let bit = match v {
            m::LostFor::Recently => values::time_since_lost::LOST_RECENTLY,
            m::LostFor::ForAWhile => values::time_since_lost::LOST_FOR_X_TIME,
        };
        f.set(Param::TimeSinceLost, bit);
    }
    if let Some(car) = p.car {
        f.set(Param::CarType, car.kind);
        f.set(Param::CarColor, car.colour);
    }
    if let Some(speed) = p.speed {
        f.set(Param::Speed, params::speed_for(speed));
    }
    if let Some(v) = p.measurement {
        let bit = match v {
            m::Measurement::Generic => values::measurement::GENERIC,
            m::Measurement::Imperial => values::measurement::IMPERIAL_ONLY,
            m::Measurement::Metric => values::measurement::METRIC_ONLY,
        };
        f.set(Param::Measurement, bit);
    }
}

fn pursuit_type(v: m::PursuitType) -> u32 {
    match v {
        m::PursuitType::GenericSpeeder => values::pursuit_type::GENERIC_SPEEDER,
        m::PursuitType::PossibleWanted => values::pursuit_type::POSSIBLE_WANTED,
        m::PursuitType::HitAndRun => values::pursuit_type::HIT_AND_RUN,
        m::PursuitType::Reckless => values::pursuit_type::RECKLESS,
        m::PursuitType::UnitRammed => values::pursuit_type::UNIT_RAMMED,
    }
}

/// The direction, the place (region and location of the road), the encounter and the addresses.
fn places(f: &mut Fill, p: &m::Params) {
    if let Some(v) = p.direction {
        let bit = match v {
            m::Direction::North => values::direction::NORTHBOUND,
            m::Direction::South => values::direction::SOUTHBOUND,
            m::Direction::East => values::direction::EASTBOUND,
            m::Direction::West => values::direction::WESTBOUND,
        };
        f.set(Param::Direction, bit);
    }
    let located = p.road.and_then(place);
    if let Some(at) = located {
        f.set(Param::LocationRegion, at.region);
        f.set(Param::Location, params::location(at.location));
    }
    if let Some(v) = p.encounter {
        let bit = match v {
            m::Encounter::First => values::encounter::FIRST_ENCOUNTER,
            m::Encounter::Subsequent => values::encounter::SUBSEQUENT_ENCOUNTER,
        };
        f.set(Param::Encounter, bit);
    }
    if !f.takes(Param::AddressGroupType) {
        return;
    }
    if let Some(battalion) = p.address_of {
        f.set(Param::AddressGroupType, address_group_of_battalion(battalion));
        return;
    }
    if p.address_of.is_none() && p.generic_address.is_none() && p.pursuit_type.is_some() {
        // An escalation with no cop to name an area: any address will do.
        f.set(Param::AddressGroupType, values::address_group_type::GENERIC_ANY);
    }
    if let Some(generic) = p.generic_address {
        let group = match (generic, located) {
            (false, Some(at)) => address_group_of_region(at.region),
            _ => values::address_group_type::GENERIC_ANY,
        };
        f.set(Param::AddressGroupType, group);
    }
}

/// Strategies, positions, backup and its arrival time.
fn strategy(f: &mut Fill, p: &m::Params) {
    if let Some(v) = p.strategy {
        let bit = match v {
            m::SelfStrategy::Pit => values::self_strategy_type::PIT,
            m::SelfStrategy::SelfPit => values::self_strategy_type::SELF_PIT,
            m::SelfStrategy::SideRam => values::self_strategy_type::SIDE_RAM,
            m::SelfStrategy::Ram => values::self_strategy_type::RAM,
            m::SelfStrategy::RollingRoadblock => values::self_strategy_type::ROLLING_ROADBLOCK,
            m::SelfStrategy::Herding => values::self_strategy_type::HERDING,
        };
        f.set(Param::SelfStrategyType, bit);
    }
    if let Some(v) = p.rolling {
        let bit = match v {
            m::RollingStrategy::Pit => values::rolling_strategy_type::PIT,
            m::RollingStrategy::Box => values::rolling_strategy_type::BOX,
            m::RollingStrategy::RollingRoadblock => values::rolling_strategy_type::ROLLING_ROADBLOCK,
            m::RollingStrategy::Herding => values::rolling_strategy_type::HERDING,
        };
        f.set(Param::RollingStrategyType, bit);
    }
    if let Some(v) = p.position {
        let bit = match v {
            m::Position::Right => values::position::RIGHT_SIDE,
            m::Position::Left => values::position::LEFT_SIDE,
            m::Position::Ahead => values::position::AHEAD,
            m::Position::Behind => values::position::BEHIND,
        };
        f.set(Param::Position, bit);
    }
    if let Some(new) = p.new_strategy {
        f.set(Param::SameNew, if new { values::same_new::NEW_STRATEGY } else { values::same_new::SAME_STRATEGY });
    }
    if let Some(v) = p.backup {
        let (asked, answered) = backup(v);
        f.set(Param::BackupType, asked);
        f.set(Param::DispBackupType, answered);
    }
    if let Some(v) = p.eta {
        let bit = match v {
            m::Eta::Secs15 => values::disp_backup_eta::_15SEC,
            m::Eta::Secs30 => values::disp_backup_eta::_30SEC,
            m::Eta::Min1 => values::disp_backup_eta::_1MIN,
            m::Eta::Min1Secs30 => values::disp_backup_eta::_1MIN30SEC,
            m::Eta::Min2 => values::disp_backup_eta::_2MIN,
            m::Eta::Over2Min => values::disp_backup_eta::_2MIN_PLUS,
        };
        f.set(Param::DispBackupEta, bit);
    }
}

/// What a cop asks for (`backup_type`) and what dispatch answers with (`disp_backup_type`) for a backup request.
fn backup(v: m::Backup) -> (u32, u32) {
    use values::{backup_type as ask, disp_backup_type as answer};
    match v {
        m::Backup::NoUnits => (ask::ALL_AVAILABLE_UNITS, answer::NO_AVAILABLE_UNITS),
        m::Backup::OneUnit => (ask::ALL_AVAILABLE_UNITS, answer::_1_UNIT),
        m::Backup::AllUnits => (ask::ALL_AVAILABLE_UNITS, answer::ALL_AVAILABLE_UNITS),
        m::Backup::AirSupport => (ask::AIR_SUPPORT, answer::AIR_SUPPORT),
        m::Backup::Rhino => (ask::RHINO_UNIT, answer::RHINO_UNIT),
        m::Backup::GroundUnits => (ask::ALL_AVAILABLE_UNITS, answer::GENERIC_GROUND_UNITS),
        m::Backup::SuperPursuit => (ask::ALL_AVAILABLE_UNITS, answer::SUPER_PURSUIT),
    }
}

/// Collisions, cops leaving, heat and the places of off-road moments.
fn incidents(f: &mut Fill, p: &m::Params) {
    if let Some(v) = p.world_object {
        let bit = match v {
            m::WorldObject::Generic => values::world_object_type::GENERIC,
            m::WorldObject::Guardrail => values::world_object_type::GUARDRAIL,
            m::WorldObject::Train => values::world_object_type::TRAIN,
            m::WorldObject::Semi => values::world_object_type::SEMI,
            m::WorldObject::GasStation => values::world_object_type::GAS_STATION,
            m::WorldObject::SpikeBelt => values::world_object_type::SPIKE_BELT,
            m::WorldObject::LowHeat => values::world_object_type::LOW_HEAT_COLLISION,
        };
        f.set(Param::WorldObjectType, bit);
    }
    if let Some(v) = p.units {
        let bit = match v {
            m::Units::One => values::num_units::ONE_UNIT_IN_PURSUIT,
            m::Units::Multiple => values::num_units::MULTIPLE_UNITS_IN_PURSUIT,
        };
        f.set(Param::NumUnits, bit);
    }
    if let Some(v) = p.bailout {
        let bit = match v {
            m::Bailout::LowIntensity => values::bailout_type::GENERIC_LOW_INTENSITY,
            m::Bailout::HeavyTraffic => values::bailout_type::HEAVY_TRAFFIC,
            m::Bailout::RoadConditions => values::bailout_type::ROAD_CONDITIONS,
            m::Bailout::Damage => values::bailout_type::DAMAGE_SUSTAINED,
            m::Bailout::HighIntensity => values::bailout_type::GENERIC_HIGH_INTENSITY,
        };
        f.set(Param::BailoutType, bit);
    }
    if let Some(other) = p.about_other {
        let bit =
            if other { values::self_other::CONTEXT_ABOUT_OTHER_UNIT } else { values::self_other::CONTEXT_ABOUT_SELF };
        f.set(Param::SelfOther, bit);
    }
    if let Some(v) = p.ev {
        let bit = match v {
            m::Ev::MultipleUnitsDown => values::ev_type::MULTIPLE_UNITS_DOWN,
            m::Ev::GasStationFire => values::ev_type::GAS_STATION_FIRE,
            m::Ev::HeliDown => values::ev_type::HELI_DOWN,
        };
        f.set(Param::EvType, bit);
    }
    if let Some((place, again)) = p.offroad {
        f.set(Param::OffroadMomentId, 1 << place);
        f.set(
            Param::FirstSubsequent,
            if again { values::first_subsequent::SUBSEQUENT_TIME } else { values::first_subsequent::FIRST_TIME },
        );
    }
    if let Some(level) = p.heat_level {
        f.set(Param::HeatLevel, params::heat_level(u32::from(level.max(2))));
    }
    if let Some(v) = p.jurisdiction {
        let bit = match v {
            m::Jurisdiction::State => values::jurisdiction::STATE,
            m::Jurisdiction::Federal => values::jurisdiction::FEDERAL,
        };
        f.set(Param::Jurisdiction, bit);
    }
}

/// Roadblocks, spike strips and the helicopter's remarks.
fn roadblocks(f: &mut Fill, p: &m::Params) {
    if let Some(v) = p.roadblock {
        let bit = match v {
            m::RoadblockKind::Generic => values::roadblock_type::ROADBLOCK_GENERIC,
            m::RoadblockKind::Spikes => values::roadblock_type::SPIKES,
            m::RoadblockKind::HeliDispatchOnly => values::roadblock_type::HELI_ROADBLOCK_DISP_ONLY,
            m::RoadblockKind::MultipleDispatchOnly => values::roadblock_type::MULTIPLE_ROADBLOCKS_DISP_ONLY,
        };
        f.set(Param::RoadblockType, bit);
    }
    if let Some(v) = p.engage {
        let bit = match v {
            m::Engage::Roadblock => values::roadblock_engage_type::ROADBLOCK,
            m::Engage::Spikes => values::roadblock_engage_type::SPIKES,
        };
        f.set(Param::RoadblockEngageType, bit);
    }
    if let Some(v) = p.spike_position {
        let bit = match v {
            m::SpikePosition::Left => values::spikebelt_position::LEFT,
            m::SpikePosition::Center => values::spikebelt_position::CENTER,
            m::SpikePosition::Right => values::spikebelt_position::RIGHT,
        };
        f.set(Param::SpikebeltPosition, bit);
    }
    if let Some(v) = p.heli_strategy {
        let bit = match v {
            m::HeliStrategy::Follow => values::heli_self_strategy_type::OFFSET_FOLLOE,
            m::HeliStrategy::Roadblock => values::heli_self_strategy_type::HELI_ROADBLOCK,
        };
        f.set(Param::HeliSelfStrategyType, bit);
    }
    if let Some(v) = p.heli_lost {
        let bit = match v {
            m::HeliLost::InTunnel => values::heli_lost_visual::IN_TUNNEL,
            m::HeliLost::Generic => values::heli_lost_visual::GENERIC,
        };
        f.set(Param::HeliLostVisual, bit);
    }
    if let Some(v) = p.heli_bailout {
        let bit = match v {
            m::HeliBailout::FlightConditions => values::heli_bailout_type::FLIGHT_CONDITIONS,
            m::HeliBailout::FuelLow => values::heli_bailout_type::FUEL_LOW,
            m::HeliBailout::DamageSustained => values::heli_bailout_type::DAMAGE_SUSTAINED,
        };
        f.set(Param::HeliBailoutType, bit);
    }
    if p.hazard.is_some() {
        f.set(Param::HeliHazardAlertType, values::heli_hazard_alert_type::APPROACHING_TUNNEL);
    }
}
