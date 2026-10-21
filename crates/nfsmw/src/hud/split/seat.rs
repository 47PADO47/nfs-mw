//! One player's HUD package bound to a [`HudState`]: the speed, the rpm bar and the gear, the nitrous gauge, the race
//! readout (place, clock, countdown, message line) and, in a drag race, the perfect-shift message.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::show::{Show, Want};
use super::tach::Tach;
use super::{Kind, Seat};
use crate::hud::gauges::{self, Nitrous};
use crate::hud::race;
use crate::hud::state::HudState;

/// The nitrous bar's mask turns from this angle (empty) down to 0 (full): 180 in the split packages, where the dial HUD
/// has 175 **[decomp]** (`NitrousGauge::Update`).
const NOS_BAR_EMPTY_DEG: f32 = 180.0;

pub struct SeatHud {
    pub seat: Seat,
    pub kind: Kind,
    pub package: PackageId,
    show: Show,
    race: race::Binding,
    tach: Tach,
    digits: [Option<ObjectRef>; 3],
    units: Option<ObjectRef>,
    nitrous: Nitrous,
    last_nos: f32,
    perfect: Option<ObjectRef>,
    last_shift: (i32, f32),
}

impl SeatHud {
    pub fn new(rt: &Runtime, package: PackageId, kind: Kind, seat: Seat) -> Self {
        let find = |name: &str| rt.find(package, fe_hash_upper(name));
        Self {
            seat,
            kind,
            package,
            show: Show::new(rt, package, kind),
            race: race::Binding::new(rt, package),
            tach: Tach::new(rt, package, kind),
            digits: [find("SPEED_DIGIT_1"), find("SPEED_DIGIT_2"), find("SPEED_DIGIT_3")],
            units: find("3rdPersonSpeedUnits"),
            nitrous: Nitrous {
                bar: rt.find(package, super::names::NITROUS_BAR),
                icon: rt.find(package, super::names::NITROUS_ICON),
                empty_deg: NOS_BAR_EMPTY_DEG,
            },
            last_nos: 0.0,
            perfect: rt.find(package, gauges::PERFECT_SHIFT),
            last_shift: (0, 0.0),
        }
    }

    /// Counts the message line's time down.
    pub fn advance(&mut self, dt: f32) {
        self.race.advance(dt);
    }

    /// Pushes the state into the objects. `map` is whether the shared minimap shows (only the first player's package has
    /// one). Call before `Runtime::update`.
    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState, map: bool) {
        self.show.update(
            rt,
            Want {
                race: s.race.is_some(),
                countdown: s.race.is_some_and(|r| r.count.is_some()),
                message: s.message.is_some() || self.race.message_active(),
                nitrous: s.has_nos,
                map,
            },
        );
        gauges::speed_readout(rt, &self.digits, self.units, s);
        self.tach.apply(rt, s);
        if s.has_nos {
            gauges::nitrous_meter(rt, self.nitrous, &mut self.last_nos, s.nos);
        }
        self.race.apply(rt, s.race);
        self.race.show_message(rt, s.message.as_ref());
        let drag_group = self.perfect.filter(|_| self.kind == Kind::Drag);
        gauges::perfect_shift(rt, drag_group, &mut self.last_shift, s);
    }
}
