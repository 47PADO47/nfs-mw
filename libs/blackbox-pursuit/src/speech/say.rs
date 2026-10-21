//! The lines a cop actor says: each fills the parameters of its sentence the way the original's actor method does, and
//! asks for the line. Dispatch's own lines are in [`say_dispatch`](super::say_dispatch).

use super::cx::Cx;
use super::input::Strategy;
use super::line::Line;
use super::params::{
    Backup, Encounter, Ev, Intensity, LostFor, Measurement, Params, Position, PursuitType, RollingStrategy,
    SelfStrategy, Unit,
};
use crate::infraction::Infraction;

/// The speeds the vehicle report steps through (the speech works in the player's unit).
const REPORT_SPEEDS: [f32; 11] = [100.0, 120.0, 140.0, 160.0, 180.0, 200.0, 220.0, 240.0, 260.0, 280.0, 300.0];
/// Kilometres per mile.
const KM_PER_MILE: f32 = 1.609_344;

impl Cx<'_> {
    pub(super) fn plain(&mut self, line: Line, id: u32) {
        self.say(line, id, Params::default());
    }

    pub(super) fn with_intensity(&mut self, line: Line, id: u32) {
        let params = Params { intensity: Some(self.intensity()), ..Params::default() };
        self.say(line, id, params);
    }

    pub(super) fn with_suspects(&mut self, line: Line, id: u32) {
        let params = Params { suspects: Some(self.suspects()), ..Params::default() };
        self.say(line, id, params);
    }

    pub fn is_heli(&self, id: u32) -> bool {
        self.roster.find(id).is_some_and(|a| a.heli)
    }

    fn own_unit(&self, id: u32) -> Option<(super::params::Battalion, u8)> {
        self.roster.find(id).map(|a| (a.battalion, a.call_sign))
    }

    // Opening exchange.

    /// The wording of the opening line from the last infraction, a recent ram, or a coin and what was said before.
    fn pursuit_type_for_stop(&mut self) -> PursuitType {
        if self.time_last_nailed_cop() < 5.0 {
            return PursuitType::UnitRammed;
        }
        if let Some(infraction) = self.core.infraction {
            return match infraction {
                Infraction::Speeding | Infraction::Racing => match self.high_intensity() {
                    true => PursuitType::PossibleWanted,
                    false => PursuitType::GenericSpeeder,
                },
                Infraction::RecklessDriving | Infraction::OffRoad => PursuitType::Reckless,
                Infraction::Assault => PursuitType::UnitRammed,
                Infraction::HitAndRun | Infraction::DamageToProperty => PursuitType::HitAndRun,
                Infraction::ResistingArrest => PursuitType::PossibleWanted,
            };
        }
        if self.high_intensity() {
            return PursuitType::PossibleWanted;
        }
        let said_before = self.chan.said(Line::AttemptVehicleStop) > 0
            || self.chan.said(Line::Spotter) > 0
            || self.chan.said(Line::HeliSpotter) > 0;
        let from_before = match said_before {
            true => PursuitType::PossibleWanted,
            false => PursuitType::GenericSpeeder,
        };
        if !self.rng.coin() {
            return from_before;
        }
        if self.core.traffic_hits_911 > 0 {
            return PursuitType::HitAndRun;
        }
        if self.core.cost_911 as f32 > 1000.0 {
            return PursuitType::Reckless;
        }
        from_before
    }

    pub fn attempt_vehicle_stop(&mut self, id: u32) {
        let Some((battalion, call_sign)) = self.own_unit(id) else { return };
        let pursuit_type = self.pursuit_type_for_stop();
        let params = Params {
            pursuit_type: Some(pursuit_type),
            suspects: Some(self.suspects()),
            battalion: Some(battalion),
            call_sign: Some(call_sign),
            ..Params::default()
        };
        self.say(Line::AttemptVehicleStop, id, params);
    }

    pub fn spotter(&mut self, id: u32) {
        if self.is_heli(id) {
            self.heli_spotter(id);
            return;
        }
        self.with_suspects(Line::Spotter, id);
    }

    pub fn spotter_reply(&mut self, id: u32) {
        self.with_suspects(Line::SpotterReply, id);
    }

    pub fn spotter_wanted(&mut self, id: u32) {
        self.plain(Line::SpotterWanted, id);
    }

    pub fn reply_911(&mut self, id: u32) {
        self.plain(Line::Unit911Reply, id);
    }

    pub fn reinitiate_pursuit(&mut self, id: u32) {
        let Some(ahead) = self.roster.find(id).map(|a| a.ahead) else { return };
        let since = self.core.time_since_last_chase;
        if since < self.tune.time_considered_lost_no_los {
            match ahead {
                true => self.spotted(id),
                false => self.regain_visual(id),
            }
            return;
        }
        let lost_for = match since < 30.0 {
            true => LostFor::Recently,
            false => LostFor::ForAWhile,
        };
        let params = Params { lost_for: Some(lost_for), suspects: Some(self.suspects()), ..Params::default() };
        self.say(Line::ReinitiatePursuit, id, params);
    }

    pub fn vehicle_report(&mut self, id: u32) {
        let Some(car) = self.snap.player.car else { return };
        if car.colour == 0 || !self.snap.player.commentable {
            return;
        }
        let metric = self.snap.player.metric;
        let speedo = match metric {
            true => self.snap.player.speed_mph * KM_PER_MILE,
            false => self.snap.player.speed_mph,
        };
        let steps = REPORT_SPEEDS.iter().take_while(|&&test| speedo >= test).count();
        let measurement = match (steps, metric) {
            (0, _) => Measurement::Generic,
            _ if self.rng.coin() => Measurement::Generic,
            (_, true) => Measurement::Metric,
            (_, false) => Measurement::Imperial,
        };
        let params =
            Params { car: Some(car), speed: Some(speedo), measurement: Some(measurement), ..Params::default() };
        self.say(Line::VehicleReport, id, params);
    }

    pub fn initiate_pursuit(&mut self, id: u32) {
        self.with_suspects(Line::InitiatePursuit, id);
    }

    pub fn location_report(&mut self, id: u32) {
        if !self.heading_valid() {
            return;
        }
        let here = self.core.current[0];
        let encounter = match self.chan.said(Line::LocationReport) > 1 {
            true => Encounter::Subsequent,
            false => Encounter::First,
        };
        let params = Params {
            suspects: Some(self.suspects()),
            direction: here.direction,
            road: Some(here.road),
            encounter: Some(encounter),
            ..Params::default()
        };
        self.say(Line::LocationReport, id, params);
    }

    pub fn suspect_behaviour(&mut self, id: u32) {
        self.with_suspects(Line::SuspectBehaviour, id);
    }

    pub fn suspect_confirmed(&mut self, id: u32) {
        self.with_suspects(Line::SuspectConfirmed, id);
    }

    pub fn driver_history(&mut self, id: u32) {
        self.plain(Line::DriverHistory, id);
    }

    pub fn primary_engage(&mut self, id: u32) {
        let Some((battalion, call_sign)) = self.own_unit(id) else { return };
        let params = Params { battalion: Some(battalion), call_sign: Some(call_sign), ..Params::default() };
        self.say(Line::PrimaryEngage, id, params);
    }

    // Strategy.

    pub fn self_strategy(&mut self, id: u32, kind: Strategy) {
        if !self.state_is(super::state::PursuitState::Active) {
            return;
        }
        let strategy = match kind {
            Strategy::Pit => {
                [SelfStrategy::Pit, SelfStrategy::SelfPit, SelfStrategy::SideRam, SelfStrategy::Ram][self.rng.below(4)]
            }
            Strategy::BoxIn | Strategy::RollingBlock | Strategy::Herd => match self.rng.coin() {
                true => SelfStrategy::RollingRoadblock,
                false => SelfStrategy::Herding,
            },
            _ => return,
        };
        self.say(Line::SelfStrategy, id, Params { strategy: Some(strategy), ..Params::default() });
    }

    pub fn initiate_strategy(&mut self, id: u32, kind: Strategy) {
        let rolling = match kind {
            Strategy::Pit => RollingStrategy::Pit,
            Strategy::BoxIn => RollingStrategy::Box,
            Strategy::RollingBlock => RollingStrategy::RollingRoadblock,
            Strategy::Herd => RollingStrategy::Herding,
            _ => return,
        };
        self.say(Line::InitiateStrategy, id, Params { rolling: Some(rolling), ..Params::default() });
    }

    /// The leader tells `subject` to take position, from the leader's own offset to the target (as the original does).
    pub fn call_to_position(&mut self, id: u32, subject: u32) {
        let (Some(me), Some((battalion, call_sign))) = (self.roster.find(id).copied(), self.own_unit(subject)) else {
            return;
        };
        let (x, z) = me.offset;
        let position = match x.abs() > z.abs() {
            true if x > 0.0 => Position::Right,
            true => Position::Left,
            false if z > 0.0 => Position::Ahead,
            false => Position::Behind,
        };
        let params = Params {
            intensity: Some(self.intensity()),
            position: Some(position),
            subject: Some(Unit { battalion, call_sign }),
            ..Params::default()
        };
        self.say(Line::CallToPosition, id, params);
    }

    pub fn call_to_position_reminder(&mut self, id: u32) {
        self.with_intensity(Line::CallToPositionReminder, id);
    }

    pub fn strategy_execute(&mut self, id: u32) {
        self.with_intensity(Line::StrategyExecute, id);
    }

    pub fn intent_to_ram(&mut self, id: u32) {
        self.with_intensity(Line::IntentToRam, id);
    }

    pub fn anticipate_success(&mut self, id: u32) {
        self.plain(Line::AnticipateSuccess, id);
    }

    pub fn anticipate_fail(&mut self, id: u32) {
        self.plain(Line::AnticipateFail, id);
    }

    pub fn outcome_fail(&mut self, id: u32, intensity: Option<Intensity>) {
        let intensity = intensity.unwrap_or_else(|| self.intensity());
        self.say(Line::OutcomeFail, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn strategy_reset(&mut self, id: u32, new_strategy: bool) {
        let params =
            Params { intensity: Some(self.intensity()), new_strategy: Some(new_strategy), ..Params::default() };
        self.say(Line::StrategyReset, id, params);
    }

    pub fn bullhorn(&mut self, id: u32) {
        if self.snap.player.commentable {
            self.plain(Line::Bullhorn, id);
        }
    }

    pub fn call_for_swarming(&mut self, id: u32) {
        self.plain(Line::CallForSwarming, id);
    }

    // Backup.

    pub fn initial_call_for_backup(&mut self, id: u32) {
        let line = match self.snap.racers_near {
            true => Line::InitialCallForBackupMulti,
            false => Line::InitialCallForBackup,
        };
        self.plain(line, id);
    }

    pub fn call_for_backup(&mut self, id: u32, backup: Backup) {
        self.say(Line::CallForBackup, id, Params { backup: Some(backup), ..Params::default() });
    }

    pub fn backup_reminder(&mut self, id: u32, backup: Backup) {
        self.say(Line::BackupReminder, id, Params { backup: Some(backup), ..Params::default() });
    }

    pub fn backup_arrives(&mut self, id: u32) {
        let Some((battalion, call_sign)) = self.own_unit(id) else { return };
        let params = Params { battalion: Some(battalion), call_sign: Some(call_sign), ..Params::default() };
        self.say(Line::BackupArrives, id, params);
    }

    pub fn unit_backup_reply(&mut self, id: u32) {
        self.plain(Line::UnitBackupReply, id);
    }

    pub fn negative_backup_reply(&mut self, id: u32) {
        self.plain(Line::NegativeBackupReply, id);
    }

    // Course of the pursuit.

    pub fn lost_suspect(&mut self, id: u32) {
        self.with_intensity(Line::LostSuspect, id);
    }

    pub fn arrest(&mut self, id: u32) {
        self.with_intensity(Line::Arrest, id);
    }

    pub fn lost_visual(&mut self, id: u32) {
        if self.is_heli(id) {
            self.heli_lost_visual(id);
            return;
        }
        self.with_intensity(Line::LostVisual, id);
    }

    pub fn regain_visual(&mut self, id: u32) {
        self.with_intensity(Line::RegainVisual, id);
    }

    pub fn bullhorn_arrest(&mut self, id: u32) {
        if self.is_heli(id) {
            self.heli_bullhorn_arrest(id);
            return;
        }
        if self.snap.player.commentable {
            self.with_intensity(Line::BullhornArrest, id);
        }
    }

    pub fn suspect_outrun(&mut self, id: u32) {
        self.with_intensity(Line::SuspectOutrun, id);
    }

    pub fn suspect_uturn(&mut self, id: u32) {
        self.with_intensity(Line::SuspectUTurn, id);
    }

    pub fn focus_change(&mut self, id: u32) {
        self.with_intensity(Line::FocusChange, id);
    }

    pub fn suspect_brake(&mut self, id: u32) {
        self.plain(Line::SuspectBrake, id);
    }

    pub fn pursuit_update_reply(&mut self, id: u32) {
        self.plain(Line::PursuitUpdateReply, id);
    }

    pub fn spotted(&mut self, id: u32) {
        let Some(speed) = self.roster.find(id).map(|a| a.speed) else { return };
        let intensity = match speed > 80.0 {
            true => Intensity::High,
            false => Intensity::Normal,
        };
        self.say(Line::Spotted, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn direction_change(&mut self, id: u32) {
        let Some(direction) = self.core.current[0].direction else { return };
        self.say(Line::DirectionHigh, id, Params { direction: Some(direction), ..Params::default() });
    }

    pub fn offroad(&mut self, id: u32, place: u8, subsequent: bool) {
        self.say(Line::OffroadMoment, id, Params { offroad: Some((place, subsequent)), ..Params::default() });
    }

    pub fn heat_jump(&mut self, id: u32, level: u8) {
        self.say(Line::HeatJump, id, Params { heat_level: Some(level), ..Params::default() });
    }

    pub fn call_for_ev(&mut self, id: u32, ev: Option<Ev>) {
        if !self.state_is(super::state::PursuitState::Active) || self.focus_is(super::state::Focus::Pursuit) {
            return;
        }
        let Some(actor) = self.roster.find(id).copied() else { return };
        let ev = match (ev, actor.heli) {
            (Some(ev), _) => ev,
            (None, true) => Ev::HeliDown,
            (None, false) if self.snap.pursuit.cops_destroyed > 1 => Ev::MultipleUnitsDown,
            (None, false) => return,
        };
        self.say(Line::CallForEv, id, Params { ev: Some(ev), ..Params::default() });
    }
}
