//! The game's messages and collisions, turned into the machine's reactions (`SoundAI::OnCollision` and the message
//! handlers of the original).

use super::actors::Pick;
use super::cx::Cx;
use super::flows::Flows;
use super::input::{Impact, Notice};
use super::params::Intensity;
use super::pursuit_flow::Cause;
use super::state::{Focus, Observation, PursuitState};
use super::sync_pursuit::Bail;

impl Cx<'_> {
    pub fn apply_notice(&mut self, flows: &mut Flows, notice: Notice) {
        match notice {
            Notice::Busted => {
                self.core.focus = Focus::Terminal;
                self.core.flags.busted = true;
            }
            Notice::AiBusted => {
                if self.core.state != PursuitState::Inactive
                    && let Some(cop) = self.roster.furthest(true)
                {
                    self.arrest(cop);
                }
            }
            Notice::Restart => {
                self.reset_pursuit(flows);
                self.clear_queue();
            }
            Notice::Infraction(infraction) => self.core.infraction = Some(infraction),
            Notice::Unspawn { cop, remove } => self.unspawn(flows, cop, remove),
            Notice::BackupRequested(backup) => flows.strategy.backup_requested(self, backup, false),
            Notice::BackupDenied(backup) => flows.strategy.backup_requested(self, backup, true),
            Notice::RoadblockDodged => flows.roadblock.dodged(self.now),
            Notice::RoadblockPosition(offset) => flows.roadblock.positioned(offset),
            Notice::HeliJoinsRoadblock => flows.roadblock.heli_joins(self),
            Notice::TireBlown => self.tire_blown(flows),
            Notice::RoadblockHit(hit) => flows.roadblock.nailed(self.now, hit),
            Notice::BlewByCop(cop) => flows.observer.blew_past(self, cop),
            Notice::Tunnel(inside) => flows.observer.tunnel(self, inside),
            Notice::GasStationExploded { by_player, at } => flows.observer.gas_station_exploded(self, by_player, at),
            Notice::PlayerHitCop { cop, impact, intensity, deliberate } => {
                self.player_hit_cop(flows, cop, impact, intensity, deliberate)
            }
            Notice::CopHitTraffic { cop } => self.cop_hit_traffic(flows, cop),
            Notice::PlayerHitWorld { intensity, crash } => self.player_hit_world(flows, intensity, crash),
            Notice::PlayerHitTraffic { intensity, semi } => self.player_hit_traffic(flows, intensity, semi),
            Notice::Crashed => self.core.t_last_crashed = self.now,
            Notice::PlayerHitSuspect => flows.observer.observe(self, Observation::SuspectHitSuspect, 0, 1.0),
            Notice::PlayerHitSpikeBelt { intensity } => self.player_hit_spike_belt(flows, intensity),
            Notice::CopHitWorld { cop, intensity, visible } => self.cop_hit_world(flows, cop, intensity, visible),
            Notice::Smashed { cost } => {
                if self.state_is(PursuitState::Inactive) {
                    self.core.cost_911 += cost;
                }
            }
            Notice::Airborne { altitude, secs } => flows.observer.airborne(self, altitude, secs),
            Notice::Said { line, speaker, params } => self.line_finished(flows, line, speaker, &params),
            Notice::ForcedBailout => self.terminate_pursuit(Bail::Forced),
        }
    }

    fn unspawn(&mut self, flows: &mut Flows, cop: u32, remove: bool) {
        if self.roster.find(cop).is_none() {
            return;
        }
        match remove {
            true => {
                flows.pursuit.on_cop_removed(cop);
                self.remove_cop(cop);
            }
            false => self.set_active(cop, false),
        }
    }

    fn line_finished(
        &mut self,
        flows: &mut Flows,
        line: super::line::Line,
        speaker: super::line::Speaker,
        params: &super::params::Params,
    ) {
        flows.pursuit.on_said(self, line, params);
        flows.strategy.on_said(self.now, line, params);
        flows.roadblock.on_said(line);
        let spikes = flows.roadblock.has_nailed_spikes();
        flows.observer.on_said(self, line, speaker, params, spikes);
    }

    fn tire_blown(&mut self, flows: &mut Flows) {
        flows.roadblock.nailed(self.now, super::input::RoadblockHit::Spikes);
        let voice = self.roster.closest(true, true).and_then(|c| self.roster.find(c)).map_or(0, |a| a.voice);
        if voice != 0 {
            flows.observer.observe(self, Observation::SuspectHitSpikeBelt, voice, 1.0);
        }
    }

    fn player_hit_spike_belt(&mut self, flows: &mut Flows, intensity: f32) {
        flows.roadblock.nailed(self.now, super::input::RoadblockHit::Object);
        let voice = self.roster.closest(false, true).and_then(|c| self.roster.find(c)).map_or(0, |a| a.voice);
        flows.observer.observe(self, Observation::SuspectHitSpikeBelt, voice, intensity);
    }

    /// The player's car hit a cop.
    fn player_hit_cop(&mut self, flows: &mut Flows, cop: u32, impact: Impact, intensity: f32, deliberate: bool) {
        let restartable = self.focus_is(Focus::Pursuit)
            && !self.core.flags.setup_restarted
            && !matches!(flows.pursuit.cause, Cause::Scripted | Cause::CopAssaulted | Cause::CopAssaultedScripted);
        if restartable && flows.pursuit.requires_restart(self) {
            self.clear_queue();
            if let Some(actor) = self.roster.find_mut(cop) {
                actor.times_rammed += 1;
                actor.last_rammed = self.now;
            }
            flows.pursuit.restart_assaulted(self);
            self.core.t_last_nailed = self.now;
            self.core.flags.setup_restarted = true;
        }
        let in_roadblock = self.roster.find(cop).is_some_and(|a| a.roadblock);
        if !in_roadblock {
            flows.observer.observe(self, Observation::CopHitSuspect(impact), 0, intensity);
        }
        if impact == Impact::None {
            return;
        }
        // The original stamps the time of the last ram before it asks whose car it was.
        self.core.t_last_nailed = self.now;
        if !deliberate || self.core.flags.cops_immune {
            return;
        }
        let Some(actor) = self.roster.find_mut(cop) else { return };
        actor.times_rammed += 1;
        actor.last_rammed = self.now;
        let actor = *actor;
        if matches!(self.core.focus, Focus::Pursuit | Focus::Terminal) || actor.destroyed || !actor.active {
            return;
        }
        self.react_to_ram(cop, impact, intensity);
    }

    /// The cop that was rammed says something rough, worse the harder the hit.
    fn react_to_ram(&mut self, cop: u32, impact: Impact, intensity: f32) {
        if intensity > 0.15 && intensity <= 0.5 {
            self.ram_reaction(cop, impact, Intensity::Normal);
            return;
        }
        if intensity > 0.75 {
            self.interrupt_violent(cop);
            return;
        }
        if intensity <= 0.5 {
            return;
        }
        if self.rng.next_f32() >= 0.67 {
            self.interrupt_expletive(cop);
            return;
        }
        self.ram_reaction(cop, impact, Intensity::High);
    }

    fn cop_hit_traffic(&mut self, flows: &mut Flows, cop: u32) {
        flows.observer.observe(self, Observation::CopHitTraffic, 0, 0.0);
        let Some(actor) = self.roster.find_mut(cop) else { return };
        actor.traffic_hits += 1;
        let active = actor.active;
        if self.state_is(PursuitState::Active) && self.focus_is(Focus::Strategy) && active {
            self.bailout_traffic(cop);
            self.random_bailout_deny(cop);
        }
    }

    fn player_hit_world(&mut self, flows: &mut Flows, intensity: f32, crash: bool) {
        if !crash {
            return;
        }
        self.core.t_last_crashed = self.now;
        if !self.state_is(PursuitState::Active) || !self.focus_is(Focus::Strategy) {
            return;
        }
        let Some(cop) = self.random_active(Pick::Any, true) else { return };
        let voice = self.roster.find(cop).map_or(0, |a| a.voice);
        flows.observer.observe(self, Observation::SuspectHitWorld, voice, intensity);
    }

    fn player_hit_traffic(&mut self, flows: &mut Flows, intensity: f32, semi: bool) {
        flows.observer.observe(self, Observation::SuspectHitTraffic, 0, intensity);
        let voice = self.roster.closest(true, true).and_then(|c| self.roster.find(c)).map_or(0, |a| a.voice);
        let hard = intensity >= self.tune.min_intensity_traffic;
        match (semi, hard) {
            (true, true) => flows.observer.observe(self, Observation::SuspectHitSemi, voice, intensity),
            (true, false) => {}
            (false, _) => flows.observer.observe(self, Observation::SuspectHitTraffic, voice, intensity),
        }
        if hard {
            self.core.traffic_hits_911 += 1;
        }
    }

    fn cop_hit_world(&mut self, flows: &mut Flows, cop: u32, intensity: f32, visible: bool) {
        let Some(actor) = self.roster.find(cop).copied() else { return };
        if !visible || !actor.active {
            return;
        }
        if intensity >= self.tune.min_intensity_cop_smash {
            match self.snap.weather {
                true => self.bailout_bad_road(cop),
                false => match actor.heli {
                    true => self.heli_bailout(cop),
                    false => self.bailout_damage(cop),
                },
            }
        }
        if actor.in_formation && self.roster.formation.len() > 1 {
            self.random_bailout_deny(cop);
        }
        flows.observer.observe(self, Observation::CopHitWorld, actor.voice, intensity);
    }
}
