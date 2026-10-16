//! The busted meter of the chase, as the world feeds it: the nearest cop on the player's height, the speed under which
//! the heat busts the player, and the timer that ends the pursuit. The timer rules are in `blackbox_pursuit::busted`;
//! this file reads the world and applies the outcome. The constants come from the Bartender file (the bust rules) and
//! the FairBusts file (the reset penalty), see `crate::settings`. Child of `police`, so it reads the police state
//! directly. Spec: `docs/specs/ai-pursuit.md` sections 4.4 and 5, `docs/specs/fairbusts-reset.md`.

use super::*;
use blackbox_pursuit::Infraction;
use blackbox_pursuit::busted::{BustInput, BustTuning, height_matches};

// The bust sequence after the meter fills (`docs/specs/busted-sequence.md`): the arrest scene, the flow and the ticket.
mod arrest;
mod flow;

pub(super) use flow::{BustFlow, BustReport, Payment};

/// The perp's class for the arrest scene chooser (0 to 3). The original reads it through the perp car's interface; the
/// port has no such value, so the bust uses class 0 until it does (`busted-sequence.md` section 4.1).
const ARREST_CLASS: u8 = 0;

/// Seconds of the post-reset blink after a manual reset: the player flashes (the reset penalty applies) while it runs.
/// A reset sets the invulnerability timer to 2 s (`docs/specs/vehicle-rigid-body.md` section 7).
pub const RESET_BLINK: f32 = 2.0;

/// The busted timer's fill and drain rates, per second: net +1 filling and -2 draining (`docs/specs/ai-pursuit.md`
/// section 5.1). Not configurable.
const FILL_RATE: f32 = 1.0;
const DRAIN_RATE: f32 = 2.0;

impl Police {
    /// The busted timer's constants: the bust rules from the Bartender file and the reset penalty from FairBusts.
    pub fn bust_tuning(&self) -> BustTuning {
        let rules = self.settings().busted;
        let penalty = self.reset_penalty();
        BustTuning {
            timeout: rules.timeout,
            reach: rules.reach,
            flashing_reach: penalty.reach,
            height_gate: rules.height_gate,
            fill_rate: FILL_RATE,
            drain_rate: DRAIN_RATE,
            flashing_multiplier: penalty.multiplier,
            flashing_speed_gate: penalty.speed_gate,
        }
    }

    /// The speed under which the player is busted at `level`, metres per second.
    pub fn bust_speed(&self, level: u32) -> f32 {
        self.settings().bust_speed_mps(level)
    }

    /// Advances the busted timer one frame. Returns `true` on the frame the player is busted; the world then ends the
    /// pursuit. Nothing runs without a chase on.
    pub(super) fn tick_busted(&mut self, dt: f32, pursuing: bool, input: BustInput) -> bool {
        if !pursuing || self.heat.is_none() {
            return false;
        }
        let tuning = self.bust_tuning();
        self.busted.update(dt, &input, &tuning)
    }

    /// The busted meter, 0 to 1 (1 once busted); it keeps its value after the chase ends, until the next chase.
    pub fn busted_meter(&self) -> f32 {
        self.busted.meter(&self.bust_tuning())
    }

    /// A manual reset (the player's reset key) starts the post-reset blink.
    pub fn note_manual_reset(&mut self) {
        self.reset_blink = RESET_BLINK;
    }

    /// Whether the player is flashing: the post-reset blink is on.
    pub fn flashing(&self) -> bool {
        self.reset_blink > 0.0
    }

    /// Whether the player has been busted in the chase (until the next chase starts). The arrest scene and the
    /// bust rewards read this.
    #[allow(dead_code)] // read by the arrest scene (E8f), not built yet
    pub fn is_busted(&self) -> bool {
        self.busted.is_busted()
    }

    /// Starts the bust sequence for the bust that just happened: the infractions so far are the ticket. The chase is in
    /// free roam (the port has no race mode yet). `roll` in `0..1` picks the arrest scene.
    pub(super) fn start_bust(&mut self, roll: f32) -> bool {
        let report = BustReport::from_log(&self.infractions, &self.settings.fines);
        let started = self.bust.start(false, ARREST_CLASS, roll, report);
        log::info!(
            "busted: arrest scene {} (the NIS player is not in the port yet)",
            self.bust.arrest_scene().unwrap_or("none")
        );
        started
    }

    /// Advances the bust sequence's clock one frame.
    pub(super) fn tick_bust(&mut self, dt: f32) {
        let _ = self.bust.tick(dt);
    }
}

impl super::super::TrafficWorld {
    /// The horizontal distance to the nearest cop whose height matches the player's (`height_gate`); infinite with
    /// none. A cop on a bridge above or a road below does not count.
    pub(super) fn nearest_matched_cop(&self, at: Vec3) -> f32 {
        let gate = self.police.bust_tuning().height_gate;
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, gate))
            .map(|car| horizontal(car.avoidable().position - at).length())
            .fold(f32::INFINITY, f32::min)
    }

    /// The position (physics space) of the nearest cop whose height matches the player's, the one
    /// `nearest_matched_cop` measures; `None` with none. The pursuit camera frames it.
    pub fn nearest_matched_cop_position(&self, at: Vec3) -> Option<Vec3> {
        let gate = self.police.bust_tuning().height_gate;
        self.cars
            .iter()
            .filter(|car| car.is_cop())
            .filter(|car| height_matches(car.avoidable().position.y - at.y, gate))
            .map(|car| car.avoidable().position)
            .min_by(|a, b| horizontal(*a - at).length().total_cmp(&horizontal(*b - at).length()))
    }

    /// The busted meter for the HUD, 0 to 1.
    pub fn busted_meter(&self) -> f32 {
        self.police.busted_meter()
    }

    /// Whether the player has been busted in the current or last chase.
    #[allow(dead_code)] // read by the arrest scene (E8f), not built yet
    pub fn is_busted(&self) -> bool {
        self.police.is_busted()
    }

    /// The bust sequence's ticket, once, when the front end should show it: this chase's infractions and their fines.
    pub fn take_bust_ticket(&mut self) -> Option<Vec<(Infraction, u32)>> {
        let report = self.police.bust.take_request()?;
        Some(report.lines.iter().map(|line| (line.infraction, line.fine)).collect())
    }

    /// The player pays the bust's fines (`marker`: with a marker). `true` when the bust took the payment.
    pub fn pay_bust(&mut self, marker: bool) -> bool {
        let payment = if marker { Payment::Marker } else { Payment::Cash };
        // The port has no career state, so no markers are in stock.
        self.police.bust.pay(payment, 0).is_ok()
    }

    /// Whether the busted overlay (`BUSTED_OVERLAY.fng`) is up: the arrest shot has run and its time has not.
    pub fn busted_overlay(&self) -> bool {
        self.police.bust.overlay_shown()
    }
}

#[cfg(test)]
mod bust_sequence_tests {
    use super::*;

    #[test]
    fn a_bust_runs_its_clock_hands_the_ticket_over_once_and_takes_the_payment() {
        let mut police = Police::default();
        assert!(police.start_bust(0.0), "a bust starts");
        assert!(!police.bust.overlay_shown());
        police.tick_bust(flow::SHOT_SECS + 0.1);
        assert!(police.bust.overlay_shown(), "the overlay follows the arrest shot");
        police.tick_bust(flow::SHOT_SECS + 0.1);
        assert!(!police.bust.overlay_shown(), "the overlay runs its time");
        assert!(police.bust.take_request().is_some(), "the ticket is up for the front end");
        assert!(police.bust.take_request().is_none(), "and handed over once");
        assert!(police.bust.pay(Payment::Cash, 0).is_ok(), "the payment is taken on the ticket");
    }
}
