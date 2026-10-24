//! The chase as the mods see it, from the traffic world's side: the requests the mods made are applied at the start of the
//! police update, and the chase is published at the middle of it, where the bust and the escape are decided (the outbox
//! and the rules of the notices are in `modding`). Child of `police`, so it reads the police and the traffic directly.

use nfsmw_modding::abi::EndMode;

use super::*;
use crate::modding::outbox;
use crate::modding::pursuit_feed::{PursuitRequest, PursuitView};

impl Police {
    /// The heat value of the chase, 0 when none is on.
    pub fn heat_value(&self) -> f32 {
        self.heat.map_or(0.0, |heat| heat.value())
    }

    /// Sets the heat of the chase to `value`, a mod's `heat_set`: it raises the heat, or scales it down, and the chase's
    /// range keeps it in bounds. Does nothing without a chase.
    pub fn set_heat(&mut self, value: f32) {
        let Some(heat) = self.heat.as_mut() else { return };
        if value > heat.value() {
            heat.raise_to(value);
            return;
        }
        if heat.value() > 0.0 {
            heat.scale(value / heat.value());
        }
    }

    /// Seconds of pursuit so far. The clock stops with the chase and keeps its value until the next one begins.
    pub fn pursuit_secs(&self) -> f32 {
        self.pursuit_time
    }
}

impl super::super::TrafficWorld {
    /// Applies the requests the mods made since the last police update: a heat to set, and a chase to end.
    pub(super) fn apply_mod_requests(&mut self) {
        for request in outbox::take_requests() {
            match request {
                PursuitRequest::SetHeat(heat) => self.police.set_heat(heat),
                PursuitRequest::End(mode) => self.end_for_mod(mode),
            }
        }
    }

    /// Publishes the chase to the mods: the snapshot the notices of this frame come from. The HUD takes the same notices
    /// (`take_pursuit_notices`).
    pub(super) fn publish_to_mods(&mut self) {
        let notices = outbox::publish(self.mod_view());
        self.hud_notices.extend(notices);
    }

    /// The chase as the mods read it this frame.
    fn mod_view(&self) -> PursuitView {
        let pursuing = self.is_pursuing();
        let cops = self.cars.iter().filter(|car| car.is_cop()).count() as u32;
        PursuitView {
            pursuing,
            heat: self.police.heat_value(),
            level: self.police.level().unwrap_or(0),
            secs: self.police.pursuit_secs(),
            cops: match pursuing {
                true => cops,
                false => 0,
            },
            wave_cops: self.pursuit.as_ref().map_or(0, |chase| chase.wave().map(|wave| wave.1).sum()),
            busted: self.police.is_busted(),
            busted_meter: self.police.busted_meter(),
            end_reason: self.police.end_reason(),
            infractions: *self.police.infractions(),
            fines: self.police.settings().fines,
        }
    }

    /// A mod ends the chase: the same outcome the game's own paths give (`EndMode::Bust` runs the bust sequence).
    fn end_for_mod(&mut self, mode: EndMode) {
        if !self.is_pursuing() {
            return;
        }
        match mode {
            EndMode::Evade => self.end_pursuit(blackbox_pursuit::EndReason::Evaded),
            EndMode::Bust => {
                self.police.start_bust(self.rng.police.next_f32());
                self.end_pursuit(blackbox_pursuit::EndReason::Busted);
            }
            EndMode::Abort => self.end_pursuit(blackbox_pursuit::EndReason::None),
        }
    }
}
