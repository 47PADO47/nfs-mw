//! Dispatch's lines and the lines with no voice, filled the way the original's dispatcher object fills them.

use super::cx::Cx;
use super::line::Line;
use super::params::{
    Backup, Battalion, Car, Eta, Jurisdiction, Params, PursuitType, RoadblockKind, SpikePosition, Unit,
};
use super::state::Observation;

impl Cx<'_> {
    fn unit_of(&self, id: u32) -> Option<Unit> {
        self.roster.find(id).map(|a| Unit { battalion: a.battalion, call_sign: a.call_sign })
    }

    pub fn disp_go_ahead(&mut self) {
        self.say_dispatch(Line::GoAhead, Params::default());
    }

    pub fn disp_ack(&mut self) {
        let params = Params { yes: Some(true), intensity: Some(super::params::Intensity::Normal), ..Params::default() };
        self.say_dispatch(Line::Acknowledge, params);
    }

    pub fn disp_arrest_reply(&mut self) {
        self.say_dispatch(Line::DispArrestReply, Params::default());
    }

    pub fn disp_time_expired(&mut self) {
        self.say_dispatch(Line::DispTimeExpired, Params::default());
    }

    pub fn disp_sub_roadblock_reply(&mut self) {
        self.say_dispatch(Line::DispSubRoadblock, Params::default());
    }

    pub fn disp_no_vehicle_description(&mut self) {
        self.say_dispatch(Line::NoVehicleDescription, Params::default());
    }

    pub fn disp_vehicle_description(&mut self) {
        let Some(car) = self.snap.player.car else { return };
        if car.colour == 0 {
            return;
        }
        let params = Params { car: Some(Car { kind: car.kind, colour: car.colour }), ..Params::default() };
        self.say_dispatch(Line::VehicleDescription, params);
    }

    pub fn disp_pursuit_update(&mut self, cop: u32) {
        let params = Params { subject: self.unit_of(cop), ..Params::default() };
        self.say_dispatch(Line::DispPursuitUpdate, params);
    }

    pub fn disp_heat_jump(&mut self, level: u8) {
        self.say_dispatch(Line::HeatJump, Params { heat_level: Some(level), ..Params::default() });
    }

    pub fn disp_jurisdiction_shift(&mut self, jurisdiction: Jurisdiction) {
        self.say_dispatch(Line::DispJurisShift, Params { jurisdiction: Some(jurisdiction), ..Params::default() });
    }

    pub fn disp_pursuit_escalation_generic(&mut self) {
        self.say_dispatch(
            Line::DispPursuitEscalationGeneric,
            Params { suspects: Some(self.suspects()), ..Params::default() },
        );
    }

    /// Dispatch reports the pursuit's heading and place, or the generic escalation when the heading is not usable.
    pub fn disp_pursuit_escalation(&mut self) {
        if !self.heading_valid() {
            self.disp_pursuit_escalation_generic();
            return;
        }
        let address_of: Option<Battalion> =
            self.roster.furthest(true).and_then(|id| self.roster.find(id)).map(|a| a.battalion);
        let pursuit_type = match self.core.last_observation {
            Observation::CopHitSuspect(_) => PursuitType::UnitRammed,
            Observation::SuspectHitSuspect | Observation::SuspectHitSpikeBelt => PursuitType::PossibleWanted,
            Observation::SuspectHitTraffic | Observation::SuspectHitTrain | Observation::SuspectHitSemi => {
                PursuitType::HitAndRun
            }
            Observation::SuspectHitWorld | Observation::SuspectHitGuardrail | Observation::SuspectHitGasStation => {
                PursuitType::Reckless
            }
            _ => PursuitType::GenericSpeeder,
        };
        let (direction, road) = self.heading_for_dispatch();
        let params = Params {
            address_of,
            pursuit_type: Some(pursuit_type),
            suspects: Some(self.suspects()),
            direction,
            road,
            ..Params::default()
        };
        self.say_dispatch(Line::DispPursuitEscalation, params);
    }

    pub fn disp_break_away(&mut self) {
        let (direction, road) = self.heading_for_dispatch();
        self.say_dispatch(Line::DispBreakAway, Params { direction, road, ..Params::default() });
    }

    pub fn disp_report_911(&mut self, pursuit_type: PursuitType) {
        let (direction, road) = self.heading_for_dispatch();
        let generic = self.rng.below(3) == 2;
        let params = Params {
            pursuit_type: Some(pursuit_type),
            suspects: Some(self.suspects()),
            encounter: Some(super::params::Encounter::First),
            direction,
            road,
            generic_address: Some(generic),
            ..Params::default()
        };
        self.say_dispatch(Line::Disp911Report, params);
    }

    pub fn disp_backup_reply(&mut self, cop: u32, yes: bool, backup: Backup) {
        let params = Params { yes: Some(yes), subject: self.unit_of(cop), backup: Some(backup), ..Params::default() };
        self.say_dispatch(Line::DispBackupReply, params);
    }

    pub fn disp_backup_update(&mut self, yes: bool) {
        self.say_dispatch(Line::DispBackupUpdate, Params { yes: Some(yes), ..Params::default() });
    }

    /// Dispatch gives the backup's arrival time (nothing when the estimate falls on a boundary or below ten seconds).
    pub fn disp_backup_eta(&mut self) {
        const BANDS: [(f32, f32, Eta); 6] = [
            (10.0, 20.0, Eta::Secs15),
            (20.0, 40.0, Eta::Secs30),
            (40.0, 75.0, Eta::Min1),
            (75.0, 100.0, Eta::Min1Secs30),
            (100.0, 140.0, Eta::Min2),
            (140.0, f32::INFINITY, Eta::Over2Min),
        ];
        let eta = self.snap.pursuit.backup_eta;
        let Some(&(_, _, band)) = BANDS.iter().find(|(low, high, _)| eta > *low && eta < *high) else { return };
        self.say_dispatch(Line::DispBackupEta, Params { eta: Some(band), ..Params::default() });
    }

    pub fn disp_roadblock_update(&mut self, yes: bool, multiple: bool) {
        let kind = match multiple {
            true => RoadblockKind::MultipleDispatchOnly,
            false => RoadblockKind::Generic,
        };
        self.say_dispatch(
            Line::DispRoadblockUpdate,
            Params { yes: Some(yes), roadblock: Some(kind), ..Params::default() },
        );
    }

    /// Dispatch answers a roadblock request; `kind` overrides the type chosen from the settings (the helicopter's).
    pub fn disp_roadblock_reply(
        &mut self,
        cop: u32,
        yes: bool,
        kind: Option<RoadblockKind>,
        multiple: bool,
        spikes: bool,
    ) {
        let kind = kind.unwrap_or(match (spikes, multiple) {
            (true, _) => RoadblockKind::Spikes,
            (false, true) => RoadblockKind::MultipleDispatchOnly,
            (false, false) => RoadblockKind::Generic,
        });
        let params = Params { yes: Some(yes), roadblock: Some(kind), subject: self.unit_of(cop), ..Params::default() };
        self.say_dispatch(Line::DispRoadblockReply, params);
    }

    // Lines with no voice.

    pub fn quadrant_forming(&mut self) {
        self.say_voiceless(Line::QuadrantForming, Params::default());
    }

    pub fn possible_suspect(&mut self) {
        self.say_voiceless(Line::PossibleSuspect, Params::default());
    }

    pub fn wrong_suspect(&mut self) {
        self.say_voiceless(Line::WrongSuspect, Params::default());
    }

    pub fn suspect_possibly_gone(&mut self) {
        self.say_voiceless(Line::SuspectPossiblyGone, Params::default());
    }

    pub fn quadrant_moving(&mut self) {
        self.say_voiceless(Line::QuadrantMoving, Params::default());
    }

    pub fn other_lead(&mut self) {
        self.say_voiceless(Line::OtherLead, Params::default());
    }

    pub fn swarming_reply(&mut self) {
        self.say_voiceless(Line::SwarmingReply, Params::default());
    }

    pub fn swarming_reply_follow(&mut self) {
        self.say_voiceless(Line::SwarmingReplyFollow, Params::default());
    }

    pub fn roadblock_warning(&mut self) {
        self.say_voiceless(Line::RoadblockWarning, Params::default());
    }

    pub fn roadblock_position(&mut self, offset: i32) {
        let position = match offset {
            o if o <= -4 => SpikePosition::Left,
            o if o > 4 => SpikePosition::Right,
            _ => SpikePosition::Center,
        };
        self.say_voiceless(Line::RoadblockPosition, Params { spike_position: Some(position), ..Params::default() });
    }

    pub fn extra_roadblock_engage(&mut self, spikes_hit: bool) {
        if self.snap.roadblock.is_none() {
            return;
        }
        let engage = match spikes_hit {
            true => super::params::Engage::Spikes,
            false => super::params::Engage::Roadblock,
        };
        self.say_voiceless(Line::ExtraRoadblockEngage, Params { engage: Some(engage), ..Params::default() });
    }

    pub fn extra_roadblock_averted(&mut self) {
        let Some(block) = self.snap.roadblock else { return };
        let engage = match block.spike_strips > 0 {
            true => super::params::Engage::Spikes,
            false => super::params::Engage::Roadblock,
        };
        self.say_voiceless(Line::ExtraRoadblockAverted, Params { engage: Some(engage), ..Params::default() });
    }

    /// The 911 call's "more details" line, from a random one of the first four cop voices. Returns the voice.
    pub fn more_details(&mut self) -> u8 {
        let voices = self.layout.cop_voices();
        let voice = voices[self.rng.below(voices.len().min(4))];
        self.emit(Line::MoreDetails, voice, Params::default(), None);
        voice
    }

    pub fn unit_911_reply(&mut self, voice: u8) {
        self.emit(Line::Unit911Reply, voice, Params::default(), None);
    }

    /// A forced bail-out line from `voice`, or from a random cop voice when the voice is 0. Returns the voice.
    pub fn misc_bailout(&mut self, voice: u8) -> u8 {
        let voice = match voice {
            0 => self.layout.cop_voices()[self.rng.below(self.layout.cop_voices().len())],
            v => v,
        };
        let params = Params { bailout: Some(super::params::Bailout::HighIntensity), ..Params::default() };
        self.emit(Line::Bailout, voice, params, None);
        voice
    }

    /// "Lost suspect" from the last speaker (or a random voice).
    pub fn misc_lost_suspect(&mut self, voice: u8) {
        let voice = match voice {
            0 => self.layout.cop_voices()[self.rng.below(self.layout.cop_voices().len())],
            v => v,
        };
        let params = Params { intensity: Some(self.intensity()), ..Params::default() };
        self.emit(Line::LostSuspect, voice, params, None);
    }
}
