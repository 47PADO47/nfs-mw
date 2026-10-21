//! The lines a cop actor says about collisions, cops leaving, roadblocks and the helicopter (see [`say`](super::say)).

use super::cx::Cx;
use super::line::Line;
use super::params::{Bailout, Direction, Intensity, Params, Units, WorldObject};

impl Cx<'_> {
    // Collisions.

    fn units(&self) -> Units {
        match self.snap.pursuit.exists && self.core.los_count > 1 {
            true => Units::Multiple,
            false => Units::One,
        }
    }

    fn world_collision(&mut self, id: u32, object: WorldObject) {
        let params = Params { world_object: Some(object), units: Some(self.units()), ..Params::default() };
        self.say(Line::CollisionWorld, id, params);
    }

    /// The player hit something of the world: the wording is the generic one once one was said or the chase is hot.
    pub fn impact_suspect_world(&mut self, id: u32) {
        let object = match self.chan.said(Line::CollisionWorld) > 0 || self.high_intensity() {
            true => WorldObject::Generic,
            false => WorldObject::LowHeat,
        };
        self.world_collision(id, object);
    }

    pub fn impact_suspect_semi(&mut self, id: u32) {
        self.world_collision(id, WorldObject::Semi);
    }

    pub fn impact_suspect_train(&mut self, id: u32) {
        self.world_collision(id, WorldObject::Train);
    }

    pub fn impact_suspect_guardrail(&mut self, id: u32) {
        self.world_collision(id, WorldObject::Guardrail);
    }

    pub fn impact_suspect_spike_belt(&mut self, id: u32) {
        self.clear_queue();
        self.world_collision(id, WorldObject::SpikeBelt);
    }

    pub fn impact_suspect_gas_station(&mut self, id: u32) {
        if !self.state_is(super::state::PursuitState::Active) {
            return;
        }
        let params =
            Params { world_object: Some(WorldObject::GasStation), units: Some(Units::One), ..Params::default() };
        self.say(Line::CollisionWorld, id, params);
    }

    pub fn impact_suspect_traffic(&mut self, id: u32, intensity: Intensity) {
        self.say(Line::CollWorldCivi, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn suspect_rollover(&mut self, id: u32, intensity: Intensity) {
        self.say(Line::CollWorldFlip, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn suspect_airborne(&mut self, id: u32, intensity: Intensity) {
        self.say(Line::CollWorldAir, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn suspect_spunout(&mut self, id: u32, intensity: Intensity) {
        self.say(Line::CollWorldSpin, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    /// The player rammed this cop: the wording follows the contact.
    pub fn ram_reaction(&mut self, id: u32, impact: super::input::Impact, intensity: Intensity) {
        use super::input::Impact;
        let line = match impact {
            Impact::RearEnd => Line::RamRearEnd,
            Impact::TBone => Line::RamTBone,
            Impact::HeadOn => Line::RamHeadOn,
            Impact::SideSwipe => Line::RamSideSwipe,
            Impact::Other | Impact::None => return,
        };
        self.say(line, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn interrupt_expletive(&mut self, id: u32) {
        self.plain(Line::InterruptExpletive, id);
    }

    pub fn interrupt_violent(&mut self, id: u32) {
        self.plain(Line::InterruptViolent, id);
    }

    pub fn interrupt_composed(&mut self, id: u32, intensity: Intensity) {
        self.say(Line::Interrupt, id, Params { intensity: Some(intensity), ..Params::default() });
    }

    pub fn ack(&mut self, id: u32) {
        let Some(destroyed) = self.roster.find(id).map(|a| a.destroyed) else { return };
        let params = Params { yes: Some(!destroyed), intensity: Some(Intensity::Normal), ..Params::default() };
        self.say(Line::Acknowledge, id, params);
    }

    // Cops leaving.

    pub fn unit_disabled(&mut self, id: u32, other: bool) {
        let params = Params { about_other: Some(other), intensity: Some(self.intensity()), ..Params::default() };
        self.say(Line::UnitDisabled, id, params);
    }

    fn bail(&mut self, id: u32, kind: Bailout) {
        if self.core.duration < self.tune.min_bailout_duration {
            return;
        }
        self.say(Line::Bailout, id, Params { bailout: Some(kind), ..Params::default() });
    }

    pub fn bailout_damage(&mut self, id: u32) {
        self.bail(id, Bailout::Damage);
    }

    pub fn bailout_low(&mut self, id: u32) {
        self.bail(id, Bailout::LowIntensity);
    }

    pub fn bailout_high(&mut self, id: u32) {
        self.bail(id, Bailout::HighIntensity);
    }

    pub fn bailout_bad_road(&mut self, id: u32) {
        self.bail(id, Bailout::RoadConditions);
    }

    pub fn bailout_traffic(&mut self, id: u32) {
        self.bail(id, Bailout::HeavyTraffic);
    }

    pub fn deny_bailout(&mut self, id: u32) {
        self.plain(Line::BailoutDeny, id);
    }

    // Roadblocks.

    pub fn call_for_roadblock(&mut self, id: u32, spikes: bool) {
        let kind = match spikes {
            true => super::params::RoadblockKind::Spikes,
            false => super::params::RoadblockKind::Generic,
        };
        self.say(Line::CallForRoadblock, id, Params { roadblock: Some(kind), ..Params::default() });
    }

    pub fn call_for_sub_roadblock(&mut self, id: u32) {
        self.plain(Line::CallForSubRoadblock, id);
    }

    pub fn roadblock_reminder(&mut self, id: u32) {
        self.plain(Line::RoadblockReminder, id);
    }

    pub fn negative_roadblock_reply(&mut self, id: u32) {
        let Some(heli) = self.roster.find(id).map(|a| a.heli) else { return };
        if !heli {
            self.plain(Line::NegativeRoadblockReply, id);
        }
    }

    pub fn roadblock_approach(&mut self, id: u32) {
        let Some(block) = self.snap.roadblock else { return };
        let kind = match block.spike_strips > 0 {
            true => super::params::RoadblockKind::Spikes,
            false => super::params::RoadblockKind::Generic,
        };
        self.say(Line::RoadblockApproach, id, Params { roadblock: Some(kind), ..Params::default() });
    }

    pub fn roadblock_engage(&mut self, id: u32, spikes_hit: bool) {
        if self.snap.roadblock.is_none() {
            return;
        }
        let engage = match spikes_hit {
            true => super::params::Engage::Spikes,
            false => super::params::Engage::Roadblock,
        };
        self.say(Line::RoadblockEngage, id, Params { engage: Some(engage), ..Params::default() });
    }

    pub fn roadblock_averted(&mut self, id: u32) {
        self.plain(Line::RoadblockAverted, id);
    }

    pub fn pursuit_approaching(&mut self, id: u32) {
        self.with_suspects(Line::PursuitApproaching, id);
    }

    // The helicopter.

    pub fn heli_self_strategy(&mut self, id: u32, kind: super::params::HeliStrategy) {
        self.say(Line::HeliSelfStrategy, id, Params { heli_strategy: Some(kind), ..Params::default() });
    }

    pub fn heli_lost_visual(&mut self, id: u32) {
        let lost = match self.core.tunnel {
            true => super::params::HeliLost::InTunnel,
            false => super::params::HeliLost::Generic,
        };
        self.say(Line::HeliLostVisual, id, Params { heli_lost: Some(lost), ..Params::default() });
    }

    fn heli_bailout_cause(&self, id: u32) -> super::params::HeliBailout {
        use super::params::HeliBailout;
        let Some(actor) = self.roster.find(id) else { return HeliBailout::FuelLow };
        if actor.health < 1.0 {
            return HeliBailout::DamageSustained;
        }
        match self.snap.weather {
            true => HeliBailout::FlightConditions,
            false => HeliBailout::FuelLow,
        }
    }

    pub fn heli_intent_to_bail(&mut self, id: u32) {
        let cause = self.heli_bailout_cause(id);
        self.say(Line::HeliIntentToBail, id, Params { heli_bailout: Some(cause), ..Params::default() });
    }

    pub fn heli_bailout(&mut self, id: u32) {
        let cause = self.heli_bailout_cause(id);
        self.say(Line::HeliBailout, id, Params { heli_bailout: Some(cause), ..Params::default() });
    }

    pub fn heli_swarming(&mut self, id: u32) {
        self.plain(Line::HeliSwarming, id);
    }

    pub fn heli_spotter(&mut self, id: u32) {
        self.plain(Line::HeliSpotter, id);
    }

    pub fn heli_hazard_alert(&mut self, id: u32) {
        let params = Params { hazard: Some(super::params::Hazard::ApproachingTunnel), ..Params::default() };
        self.say(Line::HeliHazardAlert, id, params);
    }

    pub fn heli_bullhorn_arrest(&mut self, id: u32) {
        if self.snap.player.commentable {
            self.plain(Line::HeliBullhornArrest, id);
        }
    }

    /// A direction for a line that wants one (the heading, or `None` while it is not held).
    pub fn current_direction(&self) -> Option<Direction> {
        self.core.current[0].direction
    }
}
