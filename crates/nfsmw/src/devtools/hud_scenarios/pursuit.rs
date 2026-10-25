//! The chase scenarios: one heat-3 chase, seen in each of its phases. Only what the player can fairly know is placed
//! (`hud/state/police.rs`): the cops in sight, the helicopter while it sees the player, the roadblocks once reported, and
//! where the police last saw the player.

use super::common::{HEADING, at, car, career};
use crate::hud::{
    COP, HELICOPTER, HelicopterIntel, HudEvent, HudState, LastKnown, MapBlip, PoliceIntel, PursuitHud, PursuitPhase,
    ROADBLOCK, RadarHud,
};

/// The chase's heat.
const HEAT: u32 = 3;
/// The cop cars that have joined the chase, and the ones damaged or wrecked so far.
const UNITS: u32 = 9;
const DAMAGED: u32 = 2;
const WRECKED: u32 = 2;
/// The cost to state so far.
const COST_TO_STATE: u32 = 48_600;

/// Two minutes fourteen in at 172 km/h: two cops behind in sight, the helicopter overhead with the player in sight, a
/// roadblock reported up the road. The chase has just begun for the HUD: the phase, the heat, the helicopter and the
/// roadblock are new.
pub fn active(viewport: [f32; 2]) -> HudState {
    let cops = [at(-35.0, 4.0), at(-62.0, -6.0)];
    let helicopter = at(-20.0, 30.0);
    let roadblock = at(650.0, 0.0);
    let intel = PoliceIntel {
        sightings: cops.to_vec(),
        last_known: Some(LastKnown { position: at(0.0, 0.0), age_secs: 0.0 }),
        helicopter: Some(HelicopterIntel::new(true, helicopter)),
        roadblock_reported: true,
        roadblocks: vec![roadblock],
        ..intel()
    };
    let mut blips: Vec<MapBlip> =
        cops.iter().map(|&position| MapBlip { style: COP, position, active: true, heading: Some(HEADING) }).collect();
    blips.push(MapBlip { style: HELICOPTER, position: helicopter, active: true, heading: Some(HEADING) });
    blips.push(MapBlip { style: ROADBLOCK, position: roadblock, active: false, heading: None });
    HudState {
        pursuit_phase: PursuitPhase::Active,
        pursuit: Some(PursuitHud { secs: 134.0, busted: 0.15, ..board() }),
        busted: 0.15,
        police_intel: intel,
        blips,
        cop_nearby: true,
        radar: Some(RadarHud { angle: 3.03, distance: 35.2 }),
        events: vec![
            HudEvent::PhaseChanged { from: PursuitPhase::None, to: PursuitPhase::Active },
            HudEvent::HeatUp { level: HEAT },
            HudEvent::HelicopterTracking,
            HudEvent::RoadblockReported,
        ],
        ..chase(car(172.0, 5, 6400.0, viewport))
    }
}

/// Contact lost three seconds ago at 141 km/h: no cop in sight, the helicopter still flying but blind, the search
/// working from where the police last saw the player. The HUD state has one last-known place, the police's, not one per
/// cop.
pub fn search(viewport: [f32; 2]) -> HudState {
    let intel = PoliceIntel {
        last_known: Some(LastKnown { position: at(-120.0, 0.0), age_secs: 3.0 }),
        helicopter: Some(HelicopterIntel::new(false, at(-300.0, 80.0))),
        ..intel()
    };
    HudState {
        pursuit_phase: PursuitPhase::Search,
        pursuit: Some(PursuitHud { secs: 151.0, ..board() }),
        police_intel: intel,
        events: vec![
            HudEvent::PhaseChanged { from: PursuitPhase::Active, to: PursuitPhase::Search },
            HudEvent::ContactLost,
        ],
        ..chase(car(141.0, 4, 5900.0, viewport))
    }
}

/// The cool-down meter up and 40 % done, the player parked in a hiding spot that has latched 60 % of its way.
pub fn cooldown(viewport: [f32; 2]) -> HudState {
    let intel = PoliceIntel { last_known: Some(LastKnown { position: at(-260.0, 40.0), age_secs: 14.0 }), ..intel() };
    HudState {
        pursuit_phase: PursuitPhase::Cooldown,
        pursuit: Some(PursuitHud { secs: 178.0, evade: 0.4, cooldown: true, hiding: Some(0.6), ..board() }),
        police_intel: intel,
        events: vec![
            HudEvent::PhaseChanged { from: PursuitPhase::Search, to: PursuitPhase::Cooldown },
            HudEvent::CooldownStarted,
        ],
        ..chase(car(0.0, 1, 900.0, viewport))
    }
}

/// Got away after 4:05: the cool-down full, the end card's phase held, the bounty banked.
pub fn escape(viewport: [f32; 2]) -> HudState {
    HudState {
        pursuit_phase: PursuitPhase::Escaped,
        pursuit: Some(PursuitHud { secs: 245.0, evade: 1.0, cooldown: true, ..board() }),
        career_bounty: Some(162_500),
        events: vec![
            HudEvent::PhaseChanged { from: PursuitPhase::Cooldown, to: PursuitPhase::Escaped },
            HudEvent::Escaped,
        ],
        ..chase(car(52.0, 2, 3100.0, viewport))
    }
}

/// The chase's own numbers on top of the car: the heat, the units, the career numbers and what the police know.
fn chase(state: HudState) -> HudState {
    HudState {
        vehicle_heat: HEAT as f32,
        cops_engaged: UNITS,
        cops_damaged: DAMAGED,
        police_intel: intel(),
        ..career(state)
    }
}

/// The pursuit board with the wrecked cops counted, the rest at rest.
fn board() -> PursuitHud {
    PursuitHud { cops_destroyed: WRECKED, ..PursuitHud::default() }
}

/// What the police side of the HUD always knows in a chase: the cost to state and the units involved.
fn intel() -> PoliceIntel {
    PoliceIntel { cost_to_state: COST_TO_STATE, units_involved: UNITS, ..PoliceIntel::default() }
}
