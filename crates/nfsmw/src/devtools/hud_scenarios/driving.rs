//! The free-roam scenarios: cruising, the damaged car, and the notification preemption.

use nfsmw_data::gameplay::RaceType;

use super::common::{at, car, career};
use crate::hud::{HudEvent, HudState, NavMarker, NavMarkerKind, TireCondition, VehicleCondition};

/// The frame of `notify-preempt` on which the Critical event arrives, counted from the first frame the HUD is up: half
/// a second at 60 frames a second, while the three notices of the first frame are still up.
pub const PREEMPT_FRAME: u64 = 30;

/// Cruising at 128 km/h in 4th, nitrous half full, no race and no chase, with a race start nearby still to discover
/// and a safe house on the map.
pub fn free_roam(viewport: [f32; 2]) -> HudState {
    HudState { markers: places(), ..career(car(128.0, 4, 5200.0, viewport)) }
}

/// The places on the free-roam map: an undiscovered race start ahead and to the right, a safe house behind.
fn places() -> Vec<NavMarker> {
    vec![
        NavMarker {
            kind: NavMarkerKind::Race(RaceType::Sprint),
            position: at(420.0, -60.0),
            name: Some("fixture".to_owned()),
            discovered: false,
        },
        NavMarker { kind: NavMarkerKind::SafeHouse, position: at(-380.0, 140.0), name: None, discovered: true },
    ]
}

/// Slowing at 64 km/h in 2nd: the rear-left tyre blown, the front-right punctured, heavy body damage. The car still
/// drives. The puncture's fuse is not in the HUD state, so no time to the blow-out is given.
pub fn critical_damage(viewport: [f32; 2]) -> HudState {
    let condition = VehicleCondition {
        cosmetic: Some(0.72),
        tires: [TireCondition::Sound, TireCondition::Punctured, TireCondition::Blown, TireCondition::Sound],
        disabled: false,
        destroyed: false,
    };
    HudState {
        condition: Some(condition),
        events: vec![HudEvent::TirePunctured, HudEvent::HeavyDamage],
        ..career(car(64.0, 2, 4100.0, viewport))
    }
}

/// The events of `notify-preempt` on `frame`. The first frame puts up a Low notice (a bounty milestone) and two Medium
/// ones (a checkpoint and a lap), which fill the three places; on [`PREEMPT_FRAME`] a Critical one (a roadblock) arrives and
/// displaces the least urgent, the Low one. Free roam is not intense, so the Low notice is not held back.
pub fn preempt_events(frame: u64) -> Vec<HudEvent> {
    match frame {
        0 => vec![
            HudEvent::BountyMilestone { bounty: 150_000 },
            HudEvent::Checkpoint { number: 3, total: 8 },
            HudEvent::Lap { lap: 2, laps: 3 },
        ],
        PREEMPT_FRAME => vec![HudEvent::RoadblockReported],
        _ => Vec::new(),
    }
}
